use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::One;

use crate::hint_processor::builtin_hint_processor::secp::bigint_utils::BigInt3;
use crate::hint_processor::builtin_hint_processor::secp::secp_utils::SECP_P_V2;
use crate::hint_processor::hint_processor_definition::HintReference;
use crate::math_utils::div_mod;
use crate::serde::deserialize_program::ApTracking;
use crate::stdlib::collections::HashMap;
use crate::stdlib::prelude::String;
use crate::types::exec_scope::ExecutionScopes;
use crate::vm::errors::hint_errors::HintError;
use crate::vm::vm_core::VirtualMachine;

/// Implements hint:
/// ```python
/// from starkware.cairo.common.cairo_secp.secp_utils import pack
/// SECP_P=2**255-19
///
/// x = pack(ids.x, PRIME) % SECP_P
/// ```
pub fn ed25519_is_zero_pack(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let x = BigInt3::from_var_name("x", vm, ids_data, ap_tracking)?.pack86();
    exec_scopes.insert_value("x", x.mod_floor(&SECP_P_V2));
    exec_scopes.insert_value("SECP_P", SECP_P_V2.clone());

    Ok(())
}

/// Implements hint:
/// ```python
/// from starkware.cairo.common.cairo_secp.secp_utils import pack
/// SECP_P=2**255-19
///
/// value = pack(ids.x, PRIME) % SECP_P
/// ```
pub fn ed25519_reduce(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let x = BigInt3::from_var_name("x", vm, ids_data, ap_tracking)?.pack86();
    exec_scopes.insert_value("value", x.mod_floor(&SECP_P_V2));
    exec_scopes.insert_value("SECP_P", SECP_P_V2.clone());

    Ok(())
}

/// Implements hint:
/// ```python
/// SECP_P=2**255-19
/// from starkware.python.math_utils import div_mod
///
/// value = x_inv = div_mod(1, x, SECP_P)
/// ```
pub fn ed25519_is_zero_assign_scope_vars(
    exec_scopes: &mut ExecutionScopes,
) -> Result<(), HintError> {
    let x = exec_scopes.get::<BigInt>("x")?;
    let x_inv = div_mod(&BigInt::one(), &x, &SECP_P_V2)?;
    exec_scopes.insert_value("x_inv", x_inv.clone());
    exec_scopes.insert_value("value", x_inv);
    exec_scopes.insert_value("SECP_P", SECP_P_V2.clone());

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::any_box;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::HintProcessorData;
    use crate::hint_processor::builtin_hint_processor::hint_code;
    use crate::hint_processor::builtin_hint_processor::secp::secp_utils::SECP_P_V2;
    use crate::hint_processor::hint_processor_definition::HintProcessorLogic;
    use crate::hint_processor::hint_processor_definition::HintReference;
    use crate::stdlib::collections::HashMap;
    use crate::types::exec_scope::ExecutionScopes;
    use crate::utils::test_utils::*;
    use num_bigint::BigInt;
    use num_traits::One;
    use num_traits::Zero;

    static SECP_P_D0: i128 = 77371252455336267181195245_i128;
    static SECP_P_D1: i128 = 77371252455336267181195263_i128;
    static SECP_P_D2: i128 = 9671406556917033397649407_i128;

    fn assert_is_zero_pack_ed25519_equals(x_d0: i128, x_d1: i128, x_d2: i128, expected: BigInt) {
        let ids_data = non_continuous_ids_data![("x", 0)];

        let mut vm = vm!();
        vm.run_context.fp = 0;

        vm.segments = segments![((1, 0), x_d0), ((1, 1), x_d1), ((1, 2), x_d2)];

        let mut exec_scopes = scope![];
        assert!(run_hint!(
            vm,
            ids_data,
            hint_code::IS_ZERO_PACK_ED25519,
            &mut exec_scopes
        )
        .is_ok());

        check_scope!(
            &exec_scopes,
            [("x", expected), ("SECP_P", SECP_P_V2.clone())]
        );
    }

    fn assert_reduce_ed25519_equals(x_d0: i128, x_d1: i128, x_d2: i128, expected: BigInt) {
        let ids_data = non_continuous_ids_data![("x", 0)];

        let mut vm = vm!();
        vm.run_context.fp = 0;

        vm.segments = segments![((1, 0), x_d0), ((1, 1), x_d1), ((1, 2), x_d2)];

        let mut exec_scopes = scope![];

        assert!(run_hint!(vm, ids_data, hint_code::REDUCE_ED25519, &mut exec_scopes).is_ok());

        check_scope!(
            &exec_scopes,
            [("value", expected), ("SECP_P", SECP_P_V2.clone())]
        );
    }

    #[test]
    fn test_is_zero_pack_ed25519_with_zero() {
        assert_is_zero_pack_ed25519_equals(0, 0, 0, BigInt::zero());
    }

    #[test]
    fn test_is_zero_pack_ed25519_with_secp_prime_minus_one() {
        assert_is_zero_pack_ed25519_equals(
            SECP_P_D0 - 1,
            SECP_P_D1,
            SECP_P_D2,
            SECP_P_V2.clone() - 1,
        );
    }

    #[test]
    fn test_is_zero_pack_ed25519_with_secp_prime() {
        assert_is_zero_pack_ed25519_equals(SECP_P_D0, SECP_P_D1, SECP_P_D2, BigInt::zero());
    }

    #[test]
    fn test_reduce_ed25519_with_zero() {
        assert_reduce_ed25519_equals(0, 0, 0, BigInt::zero());
    }

    #[test]
    fn test_reduce_ed25519_with_prime_minus_one() {
        assert_reduce_ed25519_equals(SECP_P_D0 - 1, SECP_P_D1, SECP_P_D2, SECP_P_V2.clone() - 1);
    }

    #[test]
    fn test_reduce_ed25519_with_prime() {
        assert_reduce_ed25519_equals(SECP_P_D0, SECP_P_D1, SECP_P_D2, BigInt::zero());
    }

    #[test]
    fn test_is_zero_assign_scope_vars_ed25519_with_one() {
        let mut vm = vm!();
        vm.run_context.fp = 0;

        let mut exec_scopes = scope![("x", BigInt::one())];

        assert!(run_hint!(
            vm,
            HashMap::default(),
            hint_code::IS_ZERO_ASSIGN_SCOPE_VARS_ED25519,
            &mut exec_scopes
        )
        .is_ok());

        check_scope!(
            &exec_scopes,
            [
                ("x_inv", BigInt::one()),
                ("value", BigInt::one()),
                ("SECP_P", SECP_P_V2.clone())
            ]
        );
    }
}
