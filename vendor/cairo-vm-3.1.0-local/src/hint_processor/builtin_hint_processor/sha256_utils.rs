use crate::stdlib::{boxed::Box, collections::HashMap, prelude::*};

use crate::Felt252;
use crate::{
    hint_processor::{
        builtin_hint_processor::hint_utils::{
            get_integer_from_var_name, get_ptr_from_var_name, insert_value_from_var_name,
        },
        hint_processor_utils::felt_to_u32,
    },
    serde::deserialize_program::ApTracking,
    types::relocatable::MaybeRelocatable,
    vm::errors::{hint_errors::HintError, vm_errors::VirtualMachineError},
    vm::vm_core::VirtualMachine,
};
use generic_array::GenericArray;
use num_traits::ToPrimitive;
use sha2::compress256;

use crate::hint_processor::hint_processor_definition::HintReference;

use super::hint_utils::get_constant_from_var_name;

const SHA256_STATE_SIZE_FELTS: usize = 8;
const BLOCK_SIZE: usize = 7;
const IV: [u32; SHA256_STATE_SIZE_FELTS] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];

pub fn sha256_input(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let n_bytes = get_integer_from_var_name("n_bytes", vm, ids_data, ap_tracking)?;
    let n_bytes = n_bytes.as_ref();

    insert_value_from_var_name(
        "full_word",
        if n_bytes >= &Felt252::from(4_i32) {
            Felt252::ONE
        } else {
            Felt252::ZERO
        },
        vm,
        ids_data,
        ap_tracking,
    )
}

/// Inner implementation of [`sha256_main_constant_input_length`] and [`sha256_main_arbitrary_input_length`]
fn sha256_main(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    constants: &HashMap<String, Felt252>,
    iv: &mut [u32; 8],
) -> Result<(), HintError> {
    let input_ptr = get_ptr_from_var_name("sha256_start", vm, ids_data, ap_tracking)?;

    // The original code gets it from `ids` in both cases, and this makes it easier
    // to implement the arbitrary length one
    let input_chunk_size_felts =
        get_constant_from_var_name("SHA256_INPUT_CHUNK_SIZE_FELTS", constants)?
            .to_usize()
            .unwrap_or(100); // Hack: enough to fail the assertion

    if input_chunk_size_felts >= 100 {
        return Err(HintError::AssertionFailed(
            "assert 0 <= _sha256_input_chunk_size_felts < 100"
                .to_string()
                .into_boxed_str(),
        ));
    }

    let mut message: Vec<u8> = Vec::with_capacity(4 * input_chunk_size_felts);

    for i in 0..input_chunk_size_felts {
        let input_element = vm.get_integer((input_ptr + i)?)?;
        let bytes = felt_to_u32(input_element.as_ref())?.to_be_bytes();
        message.extend(bytes);
    }

    let new_message = GenericArray::clone_from_slice(&message);
    compress256(iv, &[new_message]);

    let mut output: Vec<MaybeRelocatable> = Vec::with_capacity(iv.len());

    for new_state in iv {
        output.push(Felt252::from(*new_state).into());
    }

    let output_base = get_ptr_from_var_name("output", vm, ids_data, ap_tracking)?;

    vm.write_arg(output_base, &output)
        .map_err(VirtualMachineError::Memory)?;
    Ok(())
}

/* Implements hint:
from starkware.cairo.common.cairo_sha256.sha256_utils import (
    IV, compute_message_schedule, sha2_compress_function)

_sha256_input_chunk_size_felts = int(ids.SHA256_INPUT_CHUNK_SIZE_FELTS)
assert 0 <= _sha256_input_chunk_size_felts < 100

w = compute_message_schedule(memory.get_range(
    ids.sha256_start, _sha256_input_chunk_size_felts))
new_state = sha2_compress_function(IV, w)
segments.write_arg(ids.output, new_state)
 */
pub fn sha256_main_constant_input_length(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    constants: &HashMap<String, Felt252>,
) -> Result<(), HintError> {
    let mut iv = IV;
    sha256_main(vm, ids_data, ap_tracking, constants, &mut iv)
}

/* Implements hint:
from starkware.cairo.common.cairo_sha256.sha256_utils import (
    compute_message_schedule, sha2_compress_function)

_sha256_input_chunk_size_felts = int(ids.SHA256_INPUT_CHUNK_SIZE_FELTS)
assert 0 <= _sha256_input_chunk_size_felts < 100
_sha256_state_size_felts = int(ids.SHA256_STATE_SIZE_FELTS)
assert 0 <= _sha256_state_size_felts < 100
w = compute_message_schedule(memory.get_range(
    ids.sha256_start, _sha256_input_chunk_size_felts))
new_state = sha2_compress_function(memory.get_range(ids.state, _sha256_state_size_felts), w)
segments.write_arg(ids.output, new_state)
 */
pub fn sha256_main_arbitrary_input_length(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    constants: &HashMap<String, Felt252>,
) -> Result<(), HintError> {
    let iv_ptr = get_ptr_from_var_name("state", vm, ids_data, ap_tracking)?;

    let state_size_felt = get_constant_from_var_name("SHA256_STATE_SIZE_FELTS", constants)?;

    let state_size = match state_size_felt.to_usize() {
        Some(size) if size == SHA256_STATE_SIZE_FELTS => size,
        // if size is valid, but not SHA256_STATE_SIZE_FELTS, throw error
        // NOTE: in this case the python-vm fails with "not enough values to unpack" error
        Some(size) if size < 100 => {
            return Err(HintError::InvalidValue(Box::new((
                "SHA256_STATE_SIZE_FELTS",
                *state_size_felt,
                Felt252::from(SHA256_STATE_SIZE_FELTS),
            ))))
        }
        // otherwise, fails the assert
        _ => {
            return Err(HintError::AssertionFailed(
                "assert 0 <= _sha256_state_size_felts < 100"
                    .to_string()
                    .into_boxed_str(),
            ))
        }
    };

    let mut iv = vm
        .get_integer_range(iv_ptr, state_size)?
        .into_iter()
        .map(|x| felt_to_u32(x.as_ref()))
        .collect::<Result<Vec<u32>, _>>()?
        .try_into()
        .expect("size is constant");

    sha256_main(vm, ids_data, ap_tracking, constants, &mut iv)
}

pub fn sha256_finalize(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let message: Vec<u8> = vec![0; 64];

    let mut iv = IV;

    let iv_static: Vec<MaybeRelocatable> = iv.iter().map(|n| Felt252::from(*n).into()).collect();

    let new_message = GenericArray::clone_from_slice(&message);
    compress256(&mut iv, &[new_message]);

    let mut output: Vec<MaybeRelocatable> = Vec::with_capacity(SHA256_STATE_SIZE_FELTS);

    for new_state in iv {
        output.push(Felt252::from(new_state).into());
    }

    let sha256_ptr_end = get_ptr_from_var_name("sha256_ptr_end", vm, ids_data, ap_tracking)?;

    let mut padding: Vec<MaybeRelocatable> = Vec::new();
    let zero_vector_message: Vec<MaybeRelocatable> = vec![Felt252::ZERO.into(); 16];

    for _ in 0..BLOCK_SIZE - 1 {
        padding.extend_from_slice(zero_vector_message.as_slice());
        padding.extend_from_slice(iv_static.as_slice());
        padding.extend_from_slice(output.as_slice());
    }

    vm.write_arg(sha256_ptr_end, &padding)
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
                hint_code,
            },
            hint_processor_definition::{HintProcessorLogic, HintReference},
        },
        utils::test_utils::*,
        vm::vm_core::VirtualMachine,
    };
    use assert_matches::assert_matches;

    use rstest::rstest;
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    const SHA256_INPUT_CHUNK_SIZE_FELTS: usize = 16;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn sha256_input_one() {
        let mut vm = vm_with_range_check!();
        vm.segments = segments![((1, 1), 7)];
        vm.run_context.fp = 2;
        let ids_data = ids_data!["full_word", "n_bytes"];
        assert_matches!(sha256_input(&mut vm, &ids_data, &ApTracking::new()), Ok(()));

        check_memory![vm.segments.memory, ((1, 0), 1)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn sha256_input_zero() {
        let mut vm = vm_with_range_check!();
        vm.segments = segments![((1, 1), 3)];
        vm.run_context.fp = 2;
        let ids_data = ids_data!["full_word", "n_bytes"];
        assert_matches!(sha256_input(&mut vm, &ids_data, &ApTracking::new()), Ok(()));

        check_memory![vm.segments.memory, ((1, 0), 0)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn sha256_constant_input_length_ok() {
        let hint_code = hint_code::SHA256_MAIN_CONSTANT_INPUT_LENGTH;
        let mut vm = vm_with_range_check!();

        vm.segments = segments![
            ((1, 0), (2, 0)),
            ((1, 1), (3, 0)),
            ((2, 0), 22),
            ((2, 1), 22),
            ((2, 2), 22),
            ((2, 3), 22),
            ((2, 4), 22),
            ((2, 5), 22),
            ((2, 6), 22),
            ((2, 7), 22),
            ((2, 8), 22),
            ((2, 9), 22),
            ((2, 10), 22),
            ((2, 11), 22),
            ((2, 12), 22),
            ((2, 13), 22),
            ((2, 14), 22),
            ((2, 15), 22),
            ((3, 9), 0)
        ];
        vm.run_context.fp = 2;
        let ids_data = ids_data!["sha256_start", "output"];
        let constants = HashMap::from([(
            "SHA256_INPUT_CHUNK_SIZE_FELTS".to_string(),
            Felt252::from(SHA256_INPUT_CHUNK_SIZE_FELTS),
        )]);
        assert_matches!(
            run_hint!(&mut vm, ids_data, hint_code, exec_scopes_ref!(), &constants),
            Ok(())
        );

        check_memory![
            vm.segments.memory,
            ((3, 0), 3704205499_u32),
            ((3, 1), 2308112482_u32),
            ((3, 2), 3022351583_u32),
            ((3, 3), 174314172_u32),
            ((3, 4), 1762869695_u32),
            ((3, 5), 1649521060_u32),
            ((3, 6), 2811202336_u32),
            ((3, 7), 4231099170_u32)
        ];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn sha256_arbitrary_input_length_ok() {
        let hint_code = hint_code::SHA256_MAIN_ARBITRARY_INPUT_LENGTH;
        let mut vm = vm_with_range_check!();

        vm.segments = segments![
            ((1, 0), (2, 0)),
            ((1, 1), (3, 0)),
            ((1, 2), (4, 0)),
            ((2, 0), 22),
            ((2, 1), 22),
            ((2, 2), 22),
            ((2, 3), 22),
            ((2, 4), 22),
            ((2, 5), 22),
            ((2, 6), 22),
            ((2, 7), 22),
            ((2, 8), 22),
            ((2, 9), 22),
            ((2, 10), 22),
            ((2, 11), 22),
            ((2, 12), 22),
            ((2, 13), 22),
            ((2, 14), 22),
            ((2, 15), 22),
            ((3, 9), 0),
            ((4, 0), 0x6A09E667),
            ((4, 1), 0xBB67AE85),
            ((4, 2), 0x3C6EF372),
            ((4, 3), 0xA54FF53A),
            ((4, 4), 0x510E527F),
            ((4, 5), 0x9B05688C),
            ((4, 6), 0x1F83D9AB),
            ((4, 7), 0x5BE0CD18),
        ];
        vm.run_context.fp = 3;
        let ids_data = ids_data!["sha256_start", "output", "state"];
        let constants = HashMap::from([
            (
                "SHA256_INPUT_CHUNK_SIZE_FELTS".to_string(),
                Felt252::from(SHA256_INPUT_CHUNK_SIZE_FELTS),
            ),
            (
                "SHA256_STATE_SIZE_FELTS".to_string(),
                Felt252::from(SHA256_STATE_SIZE_FELTS),
            ),
        ]);
        assert_matches!(
            run_hint!(&mut vm, ids_data, hint_code, exec_scopes_ref!(), &constants),
            Ok(())
        );
        check_memory![
            vm.segments.memory,
            ((3, 0), 1676947577_u32),
            ((3, 1), 1555161467_u32),
            ((3, 2), 2679819371_u32),
            ((3, 3), 2084775296_u32),
            ((3, 4), 3059346845_u32),
            ((3, 5), 785647811_u32),
            ((3, 6), 2729325562_u32),
            ((3, 7), 2503090120_u32)
        ];
    }

    #[rstest]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    #[case(hint_code::SHA256_MAIN_CONSTANT_INPUT_LENGTH)]
    #[case(hint_code::SHA256_MAIN_ARBITRARY_INPUT_LENGTH)]
    fn sha256_invalid_chunk_size(#[case] hint_code: &str) {
        let mut vm = vm_with_range_check!();

        vm.segments = segments![
            ((1, 0), (2, 0)),
            ((1, 1), (3, 0)),
            ((1, 2), (4, 0)),
            ((4, 0), 0x6A09E667),
            ((4, 1), 0xBB67AE85),
            ((4, 2), 0x3C6EF372),
            ((4, 3), 0xA54FF53A),
            ((4, 4), 0x510E527F),
            ((4, 5), 0x9B05688C),
            ((4, 6), 0x1F83D9AB),
            ((4, 7), 0x5BE0CD18),
        ];
        vm.run_context.fp = 3;
        let ids_data = ids_data!["sha256_start", "output", "state"];
        let constants = HashMap::from([
            (
                "SHA256_INPUT_CHUNK_SIZE_FELTS".to_string(),
                Felt252::from(100),
            ),
            (
                "SHA256_STATE_SIZE_FELTS".to_string(),
                Felt252::from(SHA256_STATE_SIZE_FELTS),
            ),
        ]);
        assert_matches!(
            run_hint!(&mut vm, ids_data, hint_code, exec_scopes_ref!(), &constants),
            Err(HintError::AssertionFailed(bx)) if bx.as_ref() == "assert 0 <= _sha256_input_chunk_size_felts < 100"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn sha256_invalid_state_size() {
        let hint_code = hint_code::SHA256_MAIN_ARBITRARY_INPUT_LENGTH;
        let mut vm = vm_with_range_check!();

        vm.segments = segments![
            ((1, 0), (2, 0)),
            ((1, 1), (3, 0)),
            ((1, 2), (4, 0)),
            ((4, 0), 0x6A09E667),
            ((4, 1), 0xBB67AE85),
            ((4, 2), 0x3C6EF372),
            ((4, 3), 0xA54FF53A),
            ((4, 4), 0x510E527F),
            ((4, 5), 0x9B05688C),
            ((4, 6), 0x1F83D9AB),
            ((4, 7), 0x5BE0CD18),
        ];
        vm.run_context.fp = 3;
        let ids_data = ids_data!["sha256_start", "output", "state"];
        let constants = HashMap::from([
            (
                "SHA256_INPUT_CHUNK_SIZE_FELTS".to_string(),
                Felt252::from(SHA256_INPUT_CHUNK_SIZE_FELTS),
            ),
            ("SHA256_STATE_SIZE_FELTS".to_string(), Felt252::from(100)),
        ]);
        assert_matches!(
            run_hint!(&mut vm, ids_data, hint_code, exec_scopes_ref!(), &constants),
            Err(HintError::AssertionFailed(bx)) if bx.as_ref() == "assert 0 <= _sha256_state_size_felts < 100"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn sha256_unexpected_state_size() {
        let hint_code = hint_code::SHA256_MAIN_ARBITRARY_INPUT_LENGTH;
        let state_size = Felt252::from(9);
        let mut vm = vm_with_range_check!();

        vm.segments = segments![
            ((1, 0), (2, 0)),
            ((1, 1), (3, 0)),
            ((1, 2), (4, 0)),
            ((4, 0), 0x6A09E667),
            ((4, 1), 0xBB67AE85),
            ((4, 2), 0x3C6EF372),
            ((4, 3), 0xA54FF53A),
            ((4, 4), 0x510E527F),
            ((4, 5), 0x9B05688C),
            ((4, 6), 0x1F83D9AB),
            ((4, 7), 0x5BE0CD18),
        ];
        vm.run_context.fp = 3;
        let ids_data = ids_data!["sha256_start", "output", "state"];
        let constants = HashMap::from([
            (
                "SHA256_INPUT_CHUNK_SIZE_FELTS".to_string(),
                Felt252::from(SHA256_INPUT_CHUNK_SIZE_FELTS),
            ),
            ("SHA256_STATE_SIZE_FELTS".to_string(), state_size),
        ]);
        let expected_size = Felt252::from(SHA256_STATE_SIZE_FELTS);
        assert_matches!(
            run_hint!(&mut vm, ids_data, hint_code, exec_scopes_ref!(), &constants),
            Err(HintError::InvalidValue(bx))
                if *bx == ("SHA256_STATE_SIZE_FELTS", state_size, expected_size)
        );
    }
}
