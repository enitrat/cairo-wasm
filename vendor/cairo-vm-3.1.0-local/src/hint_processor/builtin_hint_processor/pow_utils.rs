use crate::stdlib::{boxed::Box, collections::HashMap, prelude::*};

use crate::Felt252;
use crate::{
    hint_processor::{
        builtin_hint_processor::hint_utils::{
            get_relocatable_from_var_name, insert_value_from_var_name,
        },
        hint_processor_definition::HintReference,
    },
    serde::deserialize_program::ApTracking,
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};

/*
Implements hint:
%{ ids.locs.bit = (ids.prev_locs.exp % PRIME) & 1 %}
*/
pub fn pow(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let prev_locs_exp = vm
        .get_integer(
            (get_relocatable_from_var_name("prev_locs", vm, ids_data, ap_tracking)? + 4_i32)?,
        )
        .map_err(|_| {
            HintError::IdentifierHasNoMember(Box::new(("prev_locs".to_string(), "exp".to_string())))
        })?;
    let locs_bit = prev_locs_exp.mod_floor(&Felt252::TWO.try_into().unwrap());
    insert_value_from_var_name("locs", locs_bit, vm, ids_data, ap_tracking)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::relocatable::Relocatable;

    use crate::{
        any_box,
        hint_processor::{
            builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
            builtin_hint_processor::builtin_hint_processor_definition::HintProcessorData,
            hint_processor_definition::HintProcessorLogic,
        },
        types::relocatable::MaybeRelocatable,
        utils::test_utils::*,
        vm::{errors::memory_errors::MemoryError, vm_core::VirtualMachine},
    };
    use assert_matches::assert_matches;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_pow_ok() {
        let hint_code = "ids.locs.bit = (ids.prev_locs.exp % PRIME) & 1";
        let mut vm = vm_with_range_check!();
        //Initialize ap
        vm.run_context.fp = 12;
        vm.segments = segments![((1, 11), 3)];
        let ids_data = non_continuous_ids_data![("prev_locs", -5), ("locs", 0)];
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory inserts
        check_memory![vm.segments.memory, ((1, 12), 1)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_pow_incorrect_ids() {
        let hint_code = "ids.locs.bit = (ids.prev_locs.exp % PRIME) & 1";
        let mut vm = vm_with_range_check!();
        add_segments!(vm, 2);
        //Initialize ap
        vm.run_context.ap = 11;
        //Create incorrect ids
        let ids_data = ids_data!["locs"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::UnknownIdentifier(bx)) if bx.as_ref() == "prev_locs"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_pow_incorrect_references() {
        let hint_code = "ids.locs.bit = (ids.prev_locs.exp % PRIME) & 1";
        let mut vm = vm_with_range_check!();
        add_segments!(vm, 2);
        //Initialize fp
        vm.run_context.fp = 11;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("prev_locs", -5), ("locs", -12)];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::IdentifierHasNoMember(bx)) if *bx == ("prev_locs".to_string(), "exp".to_string())
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_pow_prev_locs_exp_is_not_integer() {
        let hint_code = "ids.locs.bit = (ids.prev_locs.exp % PRIME) & 1";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 11;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("prev_locs", -5), ("locs", -12)];
        //Insert ids.prev_locs.exp into memory as a RelocatableValue
        vm.segments = segments![((1, 10), (1, 11))];
        add_segments!(vm, 1);
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::IdentifierHasNoMember(bx)) if *bx == ("prev_locs".to_string(), "exp".to_string())
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_pow_invalid_memory_insert() {
        let hint_code = "ids.locs.bit = (ids.prev_locs.exp % PRIME) & 1";
        let mut vm = vm_with_range_check!();
        //Initialize ap
        vm.run_context.fp = 11;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("prev_locs", -5), ("locs", 0)];
        //Insert ids into memory
        vm.segments = segments![((1, 10), 3), ((1, 11), 3)];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(
                MemoryError::InconsistentMemory(bx)
            )) if *bx == (Relocatable::from((1, 11)),
                    MaybeRelocatable::from(Felt252::from(3)),
                    MaybeRelocatable::from(Felt252::ONE))
        );
    }
}
