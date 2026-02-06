use crate::hint_processor::builtin_hint_processor::secp::bigint_utils::Uint512;
use crate::hint_processor::builtin_hint_processor::uint256_utils::Uint256;
use crate::stdlib::prelude::String;
use crate::Felt252;
use crate::{
    hint_processor::hint_processor_definition::HintReference, math_utils::div_mod,
    serde::deserialize_program::ApTracking, stdlib::collections::HashMap,
    vm::errors::hint_errors::HintError,
};
use num_bigint::BigInt;
use num_traits::One;

use crate::vm::vm_core::VirtualMachine;

/*
Implements hint:
%{
    def pack_512(u, num_bits_shift: int) -> int:
        limbs = (u.d0, u.d1, u.d2, u.d3)
        return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

    x = pack_512(ids.x, num_bits_shift = 128)
    p = ids.p.low + (ids.p.high << 128)
    x_inverse_mod_p = pow(x,-1, p)

    x_inverse_mod_p_split = (x_inverse_mod_p & ((1 << 128) - 1), x_inverse_mod_p >> 128)

    ids.x_inverse_mod_p.low = x_inverse_mod_p_split[0]
    ids.x_inverse_mod_p.high = x_inverse_mod_p_split[1]
%}
*/
pub fn inv_mod_p_uint512(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let x = Uint512::from_var_name("x", vm, ids_data, ap_tracking)?.pack();

    let p = Uint256::from_var_name("p", vm, ids_data, ap_tracking)?.pack();

    let x_inverse_mod_p = Felt252::from(&div_mod(
        &BigInt::one(),
        &BigInt::from(x),
        &BigInt::from(p),
    )?);

    let x_inverse_mod_p = Uint256::from(x_inverse_mod_p);
    x_inverse_mod_p.insert_from_var_name("x_inverse_mod_p", vm, ids_data, ap_tracking)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::any_box;
    use crate::felt_str;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::HintProcessorData;
    use crate::hint_processor::hint_processor_definition::HintProcessorLogic;
    use crate::types::relocatable::Relocatable;
    use crate::utils::test_utils::mayberelocatable;
    use crate::utils::test_utils::memory;
    use crate::utils::test_utils::memory_from_memory;
    use crate::utils::test_utils::memory_inner;
    use crate::Felt252;
    use crate::{
        hint_processor::builtin_hint_processor::hint_code::INV_MOD_P_UINT512,
        types::exec_scope::ExecutionScopes,
        utils::test_utils::{
            add_segments, non_continuous_ids_data, run_hint, segments, vm_with_range_check,
        },
    };
    use num_bigint::BigUint;
    use num_traits::Num;
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    fn test_pack_512() {
        assert_eq!(
            Uint512::from_values([
                Felt252::from(13123),
                Felt252::from(534354),
                Felt252::from(9901823),
                Felt252::from(7812371)
            ]).pack(),
            BigUint::from_str_radix(
                "307823090550532533958111616786199064327151160536573522012843486812312234767517005952120863393832102810613083123402814796611",
                10
            ).unwrap()
        );
        assert_eq!(
            Uint512::from_values([
                Felt252::from(13123),
                Felt252::from(534354),
                Felt252::from(9901823),
                Felt252::from(7812371)
            ]).pack(),
            BigUint::from_str_radix(
                "307823090550532533958111616786199064327151160536573522012843486812312234767517005952120863393832102810613083123402814796611",
                10
            )
            .unwrap()
        );

        assert_eq!(
            Uint512::from_values([
                Felt252::from(90812398),
                Felt252::from(55),
                Felt252::from(83127),
                Felt252::from(45312309123_u64)
            ]).pack(),
            BigUint::from_str_radix("1785395884837388090117385402351420305430103423113021825538726783888669416377532493875431795584456624829488631993250169127284718", 10).unwrap()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_inv_mod_p_uint512_ok() {
        let mut vm = vm_with_range_check!();
        add_segments!(vm, 3);

        //Initialize fp
        vm.run_context.fp = 25;

        //Create hint data
        let ids_data = non_continuous_ids_data![("x", -5), ("p", -10), ("x_inverse_mod_p", -20)];
        vm.segments = segments![
            ((1, 20), 101), //ids.x.d0
            ((1, 21), 2),   // ids.x.d1
            ((1, 22), 15),  // ids.x.d2
            ((1, 23), 61)   // ids.x.d3
                            // ((1, 15), 201385395114098847380338600778089168199),   // ids.p.low
                            // ((1, 16), 64323764613183177041862057485226039389)   // ids.p.high
        ];
        vm.insert_value(
            Relocatable::from((1, 15)),
            felt_str!("201385395114098847380338600778089168199"),
        )
        .expect("error setting ids.p");
        vm.insert_value(
            Relocatable::from((1, 16)),
            felt_str!("64323764613183177041862057485226039389"),
        )
        .expect("error setting ids.p");

        let mut exec_scopes = ExecutionScopes::new();
        //Execute the hint
        assert!(run_hint!(vm, ids_data, INV_MOD_P_UINT512, &mut exec_scopes).is_ok());

        // Check VM inserts
        assert_eq!(
            vm.get_integer(Relocatable::from((1, 5)))
                .unwrap()
                .into_owned(),
            felt_str!("80275402838848031859800366538378848249")
        );
        assert_eq!(
            vm.get_integer(Relocatable::from((1, 6)))
                .unwrap()
                .into_owned(),
            felt_str!("5810892639608724280512701676461676039")
        );
    }
}
