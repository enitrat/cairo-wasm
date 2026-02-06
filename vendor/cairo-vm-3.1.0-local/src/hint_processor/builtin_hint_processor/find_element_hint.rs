use crate::stdlib::{boxed::Box, collections::HashMap, prelude::*};
use crate::Felt252;
use crate::{
    hint_processor::{
        builtin_hint_processor::hint_utils::{
            get_integer_from_var_name, get_ptr_from_var_name, get_relocatable_from_var_name,
            insert_value_from_var_name,
        },
        hint_processor_definition::HintReference,
        hint_processor_utils::felt_to_usize,
    },
    serde::deserialize_program::ApTracking,
    types::{errors::math_errors::MathError, exec_scope::ExecutionScopes},
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};
use num_traits::ToPrimitive;

pub fn find_element(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let key = get_integer_from_var_name("key", vm, ids_data, ap_tracking)?;
    let elm_size_bigint = get_integer_from_var_name("elm_size", vm, ids_data, ap_tracking)?;
    let n_elms = get_integer_from_var_name("n_elms", vm, ids_data, ap_tracking)?;
    let array_start = get_ptr_from_var_name("array_ptr", vm, ids_data, ap_tracking)?;
    let find_element_index = exec_scopes.get::<Felt252>("find_element_index").ok();
    let elm_size = elm_size_bigint
        .to_usize()
        .ok_or_else(|| HintError::ValueOutOfRange(Box::new(*elm_size_bigint.as_ref())))?;
    if elm_size == 0 {
        return Err(HintError::ValueOutOfRange(Box::new(elm_size_bigint)));
    }

    if let Some(find_element_index_value) = find_element_index {
        let find_element_index_usize = felt_to_usize(&find_element_index_value)?;
        let found_key = vm
            .get_integer((array_start + (elm_size * find_element_index_usize))?)
            .map_err(|_| HintError::KeyNotFound)?;

        if found_key.as_ref() != key.as_ref() {
            return Err(HintError::InvalidIndex(Box::new((
                find_element_index_value,
                key,
                found_key.into_owned(),
            ))));
        }
        insert_value_from_var_name("index", find_element_index_value, vm, ids_data, ap_tracking)?;
        exec_scopes.delete_variable("find_element_index");
        Ok(())
    } else {
        if let Ok(find_element_max_size) = exec_scopes.get_ref::<Felt252>("find_element_max_size") {
            if n_elms.as_ref() > find_element_max_size {
                return Err(HintError::FindElemMaxSize(Box::new((
                    *find_element_max_size,
                    n_elms,
                ))));
            }
        }
        let n_elms_iter: u32 = n_elms
            .to_u32()
            .ok_or_else(|| MathError::Felt252ToI32Conversion(Box::new(n_elms)))?;

        for i in 0..n_elms_iter {
            let iter_key = vm
                .get_integer((array_start + (elm_size * i as usize))?)
                .map_err(|_| HintError::KeyNotFound)?;

            if iter_key.as_ref() == key.as_ref() {
                return insert_value_from_var_name(
                    "index",
                    Felt252::from(i),
                    vm,
                    ids_data,
                    ap_tracking,
                );
            }
        }

        Err(HintError::NoValueForKeyFindElement(Box::new(key)))
    }
}

pub fn search_sorted_lower(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let find_element_max_size = exec_scopes.get::<Felt252>("find_element_max_size");
    let n_elms = get_integer_from_var_name("n_elms", vm, ids_data, ap_tracking)?;
    let rel_array_ptr = get_relocatable_from_var_name("array_ptr", vm, ids_data, ap_tracking)?;
    let elm_size = get_integer_from_var_name("elm_size", vm, ids_data, ap_tracking)?;
    let key = get_integer_from_var_name("key", vm, ids_data, ap_tracking)?;

    if elm_size == Felt252::ZERO {
        return Err(HintError::ValueOutOfRange(Box::new(elm_size)));
    }

    if let Ok(find_element_max_size) = find_element_max_size {
        if n_elms > find_element_max_size {
            return Err(HintError::FindElemMaxSize(Box::new((
                find_element_max_size,
                n_elms,
            ))));
        }
    }

    let mut array_iter = vm.get_relocatable(rel_array_ptr)?;
    let n_elms_usize = n_elms.to_usize().ok_or(HintError::KeyNotFound)?;
    let elm_size_usize = elm_size.to_usize().ok_or(HintError::KeyNotFound)?;

    for i in 0..n_elms_usize {
        let value = vm.get_integer(array_iter)?;
        if value.as_ref() >= key.as_ref() {
            return insert_value_from_var_name(
                "index",
                Felt252::from(i),
                vm,
                ids_data,
                ap_tracking,
            );
        }
        array_iter.offset += elm_size_usize;
    }
    insert_value_from_var_name("index", n_elms, vm, ids_data, ap_tracking)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::string::ToString;
    use crate::types::relocatable::Relocatable;
    use crate::{
        any_box,
        hint_processor::{
            builtin_hint_processor::{
                builtin_hint_processor_definition::{BuiltinHintProcessor, HintProcessorData},
                hint_code,
            },
            hint_processor_definition::HintProcessorLogic,
        },
        types::relocatable::MaybeRelocatable,
        utils::test_utils::*,
        vm::vm_core::VirtualMachine,
    };
    use assert_matches::assert_matches;
    use num_traits::Zero;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    fn init_vm_ids_data(
        values_to_override: HashMap<String, MaybeRelocatable>,
    ) -> (VirtualMachine, HashMap<String, HintReference>) {
        let mut vm = vm!();

        const FP_OFFSET_START: usize = 4;
        vm.run_context.fp = FP_OFFSET_START;

        for _ in 0..3 {
            vm.segments.add();
        }

        let addresses = [
            Relocatable::from((1, 0)),
            Relocatable::from((1, 1)),
            Relocatable::from((1, 2)),
            Relocatable::from((1, 4)),
            Relocatable::from((2, 0)),
            Relocatable::from((2, 1)),
            Relocatable::from((2, 2)),
            Relocatable::from((2, 3)),
        ];

        let default_values = vec![
            ("array_ptr", MaybeRelocatable::from((2, 0))),
            ("elm_size", MaybeRelocatable::from(Felt252::from(2_i32))),
            ("n_elms", MaybeRelocatable::from(Felt252::from(2_i32))),
            ("key", MaybeRelocatable::from(Felt252::from(3_i32))),
            ("arr[0].a", MaybeRelocatable::from(Felt252::ONE)),
            ("arr[0].b", MaybeRelocatable::from(Felt252::from(2_i32))),
            ("arr[1].a", MaybeRelocatable::from(Felt252::from(3_i32))),
            ("arr[1].b", MaybeRelocatable::from(Felt252::from(4_i32))),
        ];

        /* array_ptr = (1,0) -> [Struct{1, 2}, Struct{3, 4}]
          elm_size = 2
          n_elms = 2
          index = None. Should become 1
          key = 3
        */

        // Build memory
        // default_values[i].0 -> contains name
        // default_values[i].1 -> contains maybe relocatable
        for (i, memory_cell) in addresses.iter().enumerate() {
            let value_to_insert = values_to_override
                .get(default_values[i].0)
                .unwrap_or(&default_values[i].1);
            vm.segments
                .memory
                .insert(*memory_cell, value_to_insert)
                .expect("Unexpected memory insert fail");
        }
        let mut ids_data = HashMap::<String, HintReference>::new();
        for (i, name) in ["array_ptr", "elm_size", "n_elms", "index", "key"]
            .iter()
            .enumerate()
        {
            ids_data.insert(
                name.to_string(),
                HintReference::new_simple(i as i32 - FP_OFFSET_START as i32),
            );
        }

        (vm, ids_data)
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn element_found_by_search() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::new());
        assert_matches!(run_hint!(vm, ids_data, hint_code::FIND_ELEMENT), Ok(()));
        check_memory![vm.segments.memory, ((1, 3), 1)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn element_found_by_oracle() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::new());
        let mut exec_scopes = scope![("find_element_index", Felt252::ONE)];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT, &mut exec_scopes),
            Ok(())
        );
        check_memory![vm.segments.memory, ((1, 3), 1)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn element_not_found_search() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "key".to_string(),
            MaybeRelocatable::from(Felt252::from(7)),
        )]));
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT),
            Err(HintError::NoValueForKeyFindElement(bx)) if *bx == Felt252::from(7)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn element_not_found_oracle() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::new());
        let mut exec_scopes = scope![("find_element_index", Felt252::from(2))];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT, &mut exec_scopes),
            Err(HintError::KeyNotFound)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn find_elm_failed_ids_get_from_mem() {
        let mut vm = vm!();
        vm.run_context.fp = 5;
        let ids_data = ids_data!["array_ptr", "elm_size", "n_elms", "index", "key"];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT),
            Err(HintError::UnknownIdentifier(bx)) if bx.as_ref() == "key"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn find_elm_not_int_elm_size() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "elm_size".to_string(),
            MaybeRelocatable::from((7, 8)),
        )]));
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT),
            Err(HintError::IdentifierNotInteger(bx)) if bx.as_ref() == "elm_size"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn find_elm_zero_elm_size() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "elm_size".to_string(),
            MaybeRelocatable::Int(Felt252::ZERO),
        )]));
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT),
            Err(HintError::ValueOutOfRange(bx)) if *bx == Felt252::ZERO
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn find_elm_negative_elm_size() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "elm_size".to_string(),
            MaybeRelocatable::Int(Felt252::from(-1)),
        )]));
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT),
            Err(HintError::ValueOutOfRange(bx)) if *bx == Felt252::from(-1)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn find_elm_not_int_n_elms() {
        let relocatable = MaybeRelocatable::from((1, 2));
        let (mut vm, ids_data) =
            init_vm_ids_data(HashMap::from([("n_elms".to_string(), relocatable)]));
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT),
            Err(HintError::IdentifierNotInteger(bx)) if bx.as_ref() == "n_elms"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn find_elm_negative_n_elms() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "n_elms".to_string(),
            MaybeRelocatable::Int(Felt252::from(-1)),
        )]));
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT),
            Err(HintError::Math(MathError::Felt252ToI32Conversion(bx))) if *bx == Felt252::from(-1)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn find_elm_empty_scope() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::new());
        assert_matches!(run_hint!(vm, ids_data, hint_code::FIND_ELEMENT), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn find_elm_n_elms_gt_max_size() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::new());
        let mut exec_scopes = scope![("find_element_max_size", Felt252::ONE)];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT, &mut exec_scopes),
            Err(HintError::FindElemMaxSize(bx)) if *bx == (Felt252::ONE, Felt252::from(2))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn find_elm_key_not_int() {
        let relocatable = MaybeRelocatable::from((1, 4));
        let (mut vm, ids_data) =
            init_vm_ids_data(HashMap::from([("key".to_string(), relocatable)]));
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT),
            Err(HintError::IdentifierNotInteger(bx)) if bx.as_ref() == "key"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn search_sorted_lower() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::new());
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::SEARCH_SORTED_LOWER),
            Ok(())
        );

        check_memory![vm.segments.memory, ((1, 3), 1)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn search_sorted_lower_no_matches() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "key".to_string(),
            MaybeRelocatable::Int(Felt252::from(7)),
        )]));
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::SEARCH_SORTED_LOWER),
            Ok(())
        );
        check_memory![vm.segments.memory, ((1, 3), 2)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn search_sorted_lower_not_int_elm_size() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "elm_size".to_string(),
            MaybeRelocatable::from((7, 8)),
        )]));
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::SEARCH_SORTED_LOWER),
            Err(HintError::IdentifierNotInteger(bx)) if bx.as_ref() == "elm_size"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn search_sorted_lower_zero_elm_size() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "elm_size".to_string(),
            MaybeRelocatable::Int(Felt252::ZERO),
        )]));
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::SEARCH_SORTED_LOWER),
            Err(HintError::ValueOutOfRange(bx)) if (*bx).is_zero()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn search_sorted_lower_not_int_n_elms() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "n_elms".to_string(),
            MaybeRelocatable::from((2, 2)),
        )]));
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::SEARCH_SORTED_LOWER),
            Err(HintError::IdentifierNotInteger(bx)) if bx.as_ref() == "n_elms"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn search_sorted_lower_empty_scope() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::new());
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::SEARCH_SORTED_LOWER),
            Ok(())
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn search_sorted_lower_n_elms_gt_max_size() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::new());
        let mut exec_scopes = scope![("find_element_max_size", Felt252::ONE)];
        assert_matches!(
            run_hint!(
                vm,
                ids_data,
                hint_code::SEARCH_SORTED_LOWER,
                &mut exec_scopes
            ),
            Err(HintError::FindElemMaxSize(bx)) if *bx == (Felt252::ONE, Felt252::from(2))
        );
    }
}
