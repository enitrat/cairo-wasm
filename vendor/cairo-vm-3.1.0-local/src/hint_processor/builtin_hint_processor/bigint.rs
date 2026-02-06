use crate::hint_processor::builtin_hint_processor::secp::bigint_utils::BigInt5;
use crate::hint_processor::builtin_hint_processor::secp::secp_utils::BASE;
use crate::math_utils::{div_mod, safe_div_bigint, signed_felt};
use crate::stdlib::collections::HashMap;
use crate::stdlib::prelude::String;
use crate::types::exec_scope::ExecutionScopes;
use crate::Felt252;
use crate::{
    hint_processor::{
        builtin_hint_processor::secp::bigint_utils::BigInt3,
        hint_processor_definition::HintReference,
    },
    serde::deserialize_program::ApTracking,
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};
use num_bigint::BigInt;
use num_traits::Signed;

use super::hint_utils::insert_value_from_var_name;

/// Implements hint:
/// ```python
/// from starkware.cairo.common.cairo_secp.secp_utils import pack
/// from starkware.cairo.common.math_utils import as_int
/// from starkware.python.math_utils import div_mod, safe_div
///
/// p = pack(ids.P, PRIME)
/// x = pack(ids.x, PRIME) + as_int(ids.x.d3, PRIME) * ids.BASE ** 3 + as_int(ids.x.d4, PRIME) * ids.BASE ** 4
/// y = pack(ids.y, PRIME)
///
/// value = res = div_mod(x, y, p)
/// ```
pub fn bigint_pack_div_mod_hint(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let p: BigInt = BigInt3::from_var_name("P", vm, ids_data, ap_tracking)?.pack86();

    let x: BigInt = {
        let x_bigint5 = BigInt5::from_var_name("x", vm, ids_data, ap_tracking)?;
        // pack only takes the first three limbs
        let x_lower = BigInt3 {
            limbs: [
                x_bigint5.limbs[0].clone(),
                x_bigint5.limbs[1].clone(),
                x_bigint5.limbs[2].clone(),
            ],
        };
        let x_lower = x_lower.pack86();
        let d3 = signed_felt(*x_bigint5.limbs[3].as_ref());
        let d4 = signed_felt(*x_bigint5.limbs[4].as_ref());
        x_lower + d3 * BigInt::from(BASE.pow(3)) + d4 * BigInt::from(BASE.pow(4))
    };
    let y: BigInt = BigInt3::from_var_name("y", vm, ids_data, ap_tracking)?.pack86();

    let res = div_mod(&x, &y, &p)?;
    exec_scopes.insert_value("res", res.clone());
    exec_scopes.insert_value("value", res);
    exec_scopes.insert_value("x", x);
    exec_scopes.insert_value("y", y);
    exec_scopes.insert_value("p", p);

    Ok(())
}

/// Implements hint:
/// ```python
/// k = safe_div(res * y - x, p)
/// value = k if k > 0 else 0 - k
/// ids.flag = 1 if k > 0 else 0
/// ```
pub fn bigint_safe_div_hint(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let res = exec_scopes.get::<BigInt>("res")?;
    let y = exec_scopes.get::<BigInt>("y")?;
    let x = exec_scopes.get::<BigInt>("x")?;
    let p = exec_scopes.get::<BigInt>("p")?;

    let k = safe_div_bigint(&(res * y - x), &p)?;
    let (value, flag) = if k.is_positive() {
        (k.clone(), Felt252::ONE)
    } else {
        (-k.clone(), Felt252::ZERO)
    };

    exec_scopes.insert_value("k", k);
    exec_scopes.insert_value("value", value);
    insert_value_from_var_name("flag", flag, vm, ids_data, ap_tracking)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::any_box;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::HintProcessorData;
    use crate::hint_processor::builtin_hint_processor::hint_code;
    use crate::hint_processor::hint_processor_definition::{HintProcessorLogic, HintReference};
    use crate::types::exec_scope::ExecutionScopes;
    use crate::utils::test_utils::*;
    use assert_matches::assert_matches;
    use num_bigint::BigInt;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    /// Input:
    /// x = UnreducedBigInt5(0x38a23ca66202c8c2a72277, 0x6730e765376ff17ea8385, 0xca1ad489ab60ea581e6c1, 0, 0);
    /// y = UnreducedBigInt3(0x20a4b46d3c5e24cda81f22, 0x967bf895824330d4273d0, 0x541e10c21560da25ada4c);
    /// p = BigInt3(0x8a03bbfd25e8cd0364141, 0x3ffffffffffaeabb739abd, 0xfffffffffffffffffffff);
    /// expected: value = res = 109567829260688255124154626727441144629993228404337546799996747905569082729709 (py int)
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_bigint_pack_div_mod_hint() {
        // Prepare the VM context:
        let ids_data = non_continuous_ids_data![
            ("x", 0), // located at `fp + 0`.
            ("y", 5), // located at `fp + 5`.
            ("P", 8)  // located at `fp + 8`.
        ];

        let mut vm = vm!();
        vm.run_context.fp = 0;
        add_segments!(vm, 11); // Alloc space for `ids.x`, `ids.y` and `ids.p`.
        vm.segments = segments![
            ((1, 0), 0x38a23ca66202c8c2a72277_i128), // x.d0
            ((1, 1), 0x6730e765376ff17ea8385_i128),  // x.d1
            ((1, 2), 0xca1ad489ab60ea581e6c1_i128),  // x.d2
            ((1, 3), 0_i128),                        // x.d3
            ((1, 4), 0_i128),                        // x.d4
            ((1, 5), 0x20a4b46d3c5e24cda81f22_i128), // y.d0
            ((1, 6), 0x967bf895824330d4273d0_i128),  // y.d1
            ((1, 7), 0x541e10c21560da25ada4c_i128),  // y.d2
            ((1, 8), 0x8a03bbfd25e8cd0364141_i128),  // P.id0
            ((1, 9), 0x3ffffffffffaeabb739abd_i128), // P.id1
            ((1, 10), 0xfffffffffffffffffffff_i128), // P.id2
        ];

        let mut exec_scopes = ExecutionScopes::new();
        assert_matches!(
            run_hint!(
                vm,
                ids_data,
                hint_code::BIGINT_PACK_DIV_MOD,
                &mut exec_scopes
            ),
            Ok(())
        );

        let expected = bigint_str!(
            "109567829260688255124154626727441144629993228404337546799996747905569082729709"
        );
        assert_matches!(exec_scopes.get::<BigInt>("res"), Ok(x) if x == expected);
        assert_matches!(exec_scopes.get::<BigInt>("value"), Ok(x) if x == expected);
        assert_matches!(exec_scopes.get::<BigInt>("y"), Ok(x) if x == bigint_str!("38047400353360331012910998489219098987968251547384484838080352663220422975266"));
        assert_matches!(exec_scopes.get::<BigInt>("x"), Ok(x) if x == bigint_str!("91414600319290532004473480113251693728834511388719905794310982800988866814583"));
        assert_matches!(exec_scopes.get::<BigInt>("p"), Ok(x) if x == bigint_str!("115792089237316195423570985008687907852837564279074904382605163141518161494337"));
    }

    /// Input:
    /// res = 109567829260688255124154626727441144629993228404337546799996747905569082729709
    /// y = 38047400353360331012910998489219098987968251547384484838080352663220422975266
    /// x = 91414600319290532004473480113251693728834511388719905794310982800988866814583
    /// p = 115792089237316195423570985008687907852837564279074904382605163141518161494337
    /// Output:
    /// k = 36002209591245282109880156842267569109802494162594623391338581162816748840003
    /// value = 36002209591245282109880156842267569109802494162594623391338581162816748840003
    /// ids.flag = 1
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_bigint_safe_div_hint() {
        let mut exec_scopes = ExecutionScopes::new();
        exec_scopes.insert_value(
            "res",
            bigint_str!(
                "109567829260688255124154626727441144629993228404337546799996747905569082729709"
            ),
        );
        exec_scopes.insert_value(
            "x",
            bigint_str!(
                "91414600319290532004473480113251693728834511388719905794310982800988866814583"
            ),
        );
        exec_scopes.insert_value(
            "y",
            bigint_str!(
                "38047400353360331012910998489219098987968251547384484838080352663220422975266"
            ),
        );
        exec_scopes.insert_value(
            "p",
            bigint_str!(
                "115792089237316195423570985008687907852837564279074904382605163141518161494337"
            ),
        );

        let mut vm = vm!();
        let ids_data = non_continuous_ids_data![("flag", 0)];
        vm.run_context.fp = 0;
        add_segments!(vm, 2); // Alloc space for `flag`

        assert_matches!(
            run_hint!(vm, ids_data, hint_code::BIGINT_SAFE_DIV, &mut exec_scopes),
            Ok(())
        );
        assert_matches!(exec_scopes.get::<BigInt>("k"), Ok(x) if x == bigint_str!("36002209591245282109880156842267569109802494162594623391338581162816748840003"));
        assert_matches!(exec_scopes.get::<BigInt>("value"), Ok(x) if x == bigint_str!("36002209591245282109880156842267569109802494162594623391338581162816748840003"));

        check_memory![vm.segments.memory, ((1, 0), 1)];
        // let flag_result = get_integer_from_var_name("flag", vm, ids_data, ap_tracking);
        // assert!(flag_result.is_ok());
        // assert_eq!(flag_result.unwrap().as_ref(), Felt252::ONE);
    }
}
