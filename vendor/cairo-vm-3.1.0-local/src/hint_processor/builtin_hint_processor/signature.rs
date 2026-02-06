use crate::stdlib::{boxed::Box, collections::HashMap, prelude::*};

use crate::{
    hint_processor::{
        builtin_hint_processor::hint_utils::{get_integer_from_var_name, get_ptr_from_var_name},
        hint_processor_definition::HintReference,
    },
    serde::deserialize_program::ApTracking,
    types::instance_definitions::ecdsa_instance_def::CELLS_PER_SIGNATURE,
    vm::{
        errors::{hint_errors::HintError, vm_errors::VirtualMachineError},
        vm_core::VirtualMachine,
    },
};

pub fn verify_ecdsa_signature(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let signature_r = get_integer_from_var_name("signature_r", vm, ids_data, ap_tracking)?;
    let signature_s = get_integer_from_var_name("signature_s", vm, ids_data, ap_tracking)?;
    let ecdsa_ptr = get_ptr_from_var_name("ecdsa_ptr", vm, ids_data, ap_tracking)?;
    let ecdsa_builtin = &mut vm.get_signature_builtin()?;
    if ecdsa_ptr.segment_index != ecdsa_builtin.base() as isize {
        return Err(HintError::AddSignatureWrongEcdsaPtr(Box::new(ecdsa_ptr)));
    }
    if !num_integer::Integer::is_multiple_of(&ecdsa_ptr.offset, &(CELLS_PER_SIGNATURE as usize)) {
        return Err(HintError::AddSignatureNotAPublicKey(Box::new(ecdsa_ptr)));
    }
    ecdsa_builtin
        .add_signature(ecdsa_ptr, &(signature_r, signature_s))
        .map_err(VirtualMachineError::Memory)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        any_box,
        hint_processor::{
            builtin_hint_processor::{
                builtin_hint_processor_definition::{BuiltinHintProcessor, HintProcessorData},
                hint_code::VERIFY_ECDSA_SIGNATURE,
            },
            hint_processor_definition::HintProcessorLogic,
        },
        utils::test_utils::*,
        vm::runners::builtin_runner::SignatureBuiltinRunner,
    };
    use assert_matches::assert_matches;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_ecdsa_signature_valid() {
        let mut vm = vm!();
        vm.builtin_runners = vec![SignatureBuiltinRunner::new(Some(512), true).into()];
        vm.segments = segments![
            ((1, 0), (0, 0)),
            (
                (1, 1),
                (
                    "3086480810278599376317923499561306189851900463386393948998357832163236918254",
                    10
                )
            ),
            (
                (1, 2),
                (
                    "598673427589502599949712887611119751108407514580626464031881322743364689811",
                    10
                )
            )
        ];
        vm.run_context.fp = 3;
        let ids_data = ids_data!["ecdsa_ptr", "signature_r", "signature_s"];
        assert_matches!(run_hint!(vm, ids_data, VERIFY_ECDSA_SIGNATURE), Ok(()));
    }

    #[test]
    fn verify_ecdsa_signature_invalid_ecdsa_ptr() {
        let mut vm = vm!();
        vm.builtin_runners = vec![SignatureBuiltinRunner::new(Some(512), true).into()];
        vm.segments = segments![
            ((1, 0), (3, 0)),
            (
                (1, 1),
                (
                    "3086480810278599376317923499561306189851900463386393948998357832163236918254",
                    10
                )
            ),
            (
                (1, 2),
                (
                    "598673427589502599949712887611119751108407514580626464031881322743364689811",
                    10
                )
            )
        ];
        vm.run_context.fp = 3;
        let ids_data = ids_data!["ecdsa_ptr", "signature_r", "signature_s"];
        assert_matches!(run_hint!(vm, ids_data, VERIFY_ECDSA_SIGNATURE), Err(HintError::AddSignatureWrongEcdsaPtr(bx)) if *bx == (3,0).into());
    }

    #[test]
    fn verify_ecdsa_signature_invalid_input_cell() {
        let mut vm = vm!();
        vm.builtin_runners = vec![SignatureBuiltinRunner::new(Some(512), true).into()];
        vm.segments = segments![
            ((1, 0), (0, 3)),
            (
                (1, 1),
                (
                    "3086480810278599376317923499561306189851900463386393948998357832163236918254",
                    10
                )
            ),
            (
                (1, 2),
                (
                    "598673427589502599949712887611119751108407514580626464031881322743364689811",
                    10
                )
            )
        ];
        vm.run_context.fp = 3;
        let ids_data = ids_data!["ecdsa_ptr", "signature_r", "signature_s"];
        assert_matches!(run_hint!(vm, ids_data, VERIFY_ECDSA_SIGNATURE), Err(HintError::AddSignatureNotAPublicKey(bx)) if *bx == (0,3).into());
    }
}
