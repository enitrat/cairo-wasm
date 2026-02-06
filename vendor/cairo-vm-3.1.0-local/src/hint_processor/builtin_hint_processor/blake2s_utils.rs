use crate::hint_processor::hint_processor_utils::felt_to_usize;
use crate::stdlib::{borrow::Cow, collections::HashMap, prelude::*};

use crate::types::errors::math_errors::MathError;
use crate::Felt252;
use crate::{
    hint_processor::{
        builtin_hint_processor::{
            blake2s_hash::{blake2s_compress, IV},
            hint_utils::{get_ptr_from_var_name, get_relocatable_from_var_name},
        },
        hint_processor_definition::HintReference,
        hint_processor_utils::felt_to_u32,
    },
    math_utils::pow2_const_nz,
    serde::deserialize_program::ApTracking,
    types::relocatable::{MaybeRelocatable, Relocatable},
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};

use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::ToPrimitive;

use super::hint_utils::{get_integer_from_var_name, insert_value_into_ap};

fn get_fixed_size_u32_array<const T: usize>(
    h_range: &Vec<Cow<Felt252>>,
) -> Result<[u32; T], HintError> {
    let mut u32_vec = Vec::<u32>::with_capacity(h_range.len());
    for num in h_range {
        u32_vec.push(num.to_u32().ok_or(HintError::BigintToU32Fail)?);
    }
    u32_vec
        .try_into()
        .map_err(|_| HintError::FixedSizeArrayFail(T))
}

fn get_maybe_relocatable_array_from_u32(array: &Vec<u32>) -> Vec<MaybeRelocatable> {
    let mut new_array = Vec::<MaybeRelocatable>::with_capacity(array.len());
    for element in array {
        new_array.push(MaybeRelocatable::from(Felt252::from(*element)));
    }
    new_array
}

/*Helper function for the Cairo blake2s() implementation.
Computes the blake2s compress function and fills the value in the right position.
output_ptr should point to the middle of an instance, right after initial_state, message, t, f,
which should all have a value at this point, and right before the output portion which will be
written by this function.*/
fn compute_blake2s_func(vm: &mut VirtualMachine, output_ptr: Relocatable) -> Result<(), HintError> {
    let h = get_fixed_size_u32_array::<8>(&vm.get_integer_range((output_ptr - 26)?, 8)?)?;
    let message = get_fixed_size_u32_array::<16>(&vm.get_integer_range((output_ptr - 18)?, 16)?)?;
    let t = felt_to_u32(vm.get_integer((output_ptr - 2)?)?.as_ref())?;
    let f = felt_to_u32(vm.get_integer((output_ptr - 1)?)?.as_ref())?;
    let new_state =
        get_maybe_relocatable_array_from_u32(&blake2s_compress(&h, &message, t, 0, f, 0));
    vm.load_data(output_ptr, &new_state)
        .map_err(HintError::Memory)?;
    Ok(())
}

/* Implements hint:
   from starkware.cairo.common.cairo_blake2s.blake2s_utils import compute_blake2s_func
   compute_blake2s_func(segments=segments, output_ptr=ids.output)
*/
pub fn compute_blake2s(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let output = get_ptr_from_var_name("output", vm, ids_data, ap_tracking)?;
    compute_blake2s_func(vm, output)
}

/* Implements Hint:
    # Add dummy pairs of input and output.
    from starkware.cairo.common.cairo_blake2s.blake2s_utils import IV, blake2s_compress

    _n_packed_instances = int(ids.N_PACKED_INSTANCES)
    assert 0 <= _n_packed_instances < 20
    _blake2s_input_chunk_size_felts = int(ids.INPUT_BLOCK_FELTS)
    assert 0 <= _blake2s_input_chunk_size_felts < 100

    message = [0] * _blake2s_input_chunk_size_felts
    modified_iv = [IV[0] ^ 0x01010020] + IV[1:]
    output = blake2s_compress(
        message=message,
        h=modified_iv,
        t0=0,
        t1=0,
        f0=0xffffffff,
        f1=0,
    )
    padding = (modified_iv + message + [0, 0xffffffff] + output) * (_n_packed_instances - 1)
    segments.write_arg(ids.blake2s_ptr_end, padding)
*/
pub fn finalize_blake2s(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    const N_PACKED_INSTANCES: usize = 7;
    let blake2s_ptr_end = get_ptr_from_var_name("blake2s_ptr_end", vm, ids_data, ap_tracking)?;
    let message: [u32; 16] = [0; 16];
    let mut modified_iv = IV;
    modified_iv[0] = IV[0] ^ 0x01010020;
    let output = blake2s_compress(&modified_iv, &message, 0, 0, 0xffffffff, 0);
    let mut padding = modified_iv.to_vec();
    padding.extend(message);
    padding.extend([0, 0xffffffff]);
    padding.extend(output);
    let padding = padding.as_slice();
    let mut full_padding = Vec::<u32>::with_capacity(padding.len() * N_PACKED_INSTANCES);
    for _ in 0..N_PACKED_INSTANCES - 1 {
        full_padding.extend_from_slice(padding);
    }
    let data = get_maybe_relocatable_array_from_u32(&full_padding);
    vm.load_data(blake2s_ptr_end, &data)
        .map_err(HintError::Memory)?;
    Ok(())
}

/* Implements Hint:
        # Add dummy pairs of input and output.
        from starkware.cairo.common.cairo_blake2s.blake2s_utils import IV, blake2s_compress

        _n_packed_instances = int(ids.N_PACKED_INSTANCES)
        assert 0 <= _n_packed_instances < 20
        _blake2s_input_chunk_size_felts = int(ids.BLAKE2S_INPUT_CHUNK_SIZE_FELTS)
        assert 0 <= _blake2s_input_chunk_size_felts < 100

        message = [0] * _blake2s_input_chunk_size_felts
        modified_iv = [IV[0] ^ 0x01010020] + IV[1:]
        output = blake2s_compress(
            message=message,
            h=modified_iv,
            t0=0,
            t1=0,
            f0=0xffffffff,
            f1=0,
        )
        padding = (message + modified_iv + [0, 0xffffffff] + output) * (_n_packed_instances - 1)
        segments.write_arg(ids.blake2s_ptr_end, padding)
*/
pub fn finalize_blake2s_v3(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    const N_PACKED_INSTANCES: usize = 7;
    let blake2s_ptr_end = get_ptr_from_var_name("blake2s_ptr_end", vm, ids_data, ap_tracking)?;
    let message: [u32; 16] = [0; 16];
    let mut modified_iv = IV;
    modified_iv[0] = IV[0] ^ 0x01010020;
    let output = blake2s_compress(&modified_iv, &message, 0, 0, 0xffffffff, 0);
    let mut padding = message.to_vec();
    padding.extend(modified_iv);
    padding.extend([0, 0xffffffff]);
    padding.extend(output);
    let padding = padding.as_slice();
    let mut full_padding = Vec::<u32>::with_capacity(padding.len() * N_PACKED_INSTANCES);
    for _ in 0..N_PACKED_INSTANCES - 1 {
        full_padding.extend_from_slice(padding);
    }
    let data = get_maybe_relocatable_array_from_u32(&full_padding);
    vm.load_data(blake2s_ptr_end, &data)
        .map_err(HintError::Memory)?;
    Ok(())
}

/* Implements Hint:
    B = 32
    MASK = 2 ** 32 - 1
    segments.write_arg(ids.data, [(ids.low >> (B * i)) & MASK for i in range(4)])
    segments.write_arg(ids.data + 4, [(ids.high >> (B * i)) & MASK for i in range(4)])
*/
pub fn blake2s_add_uint256(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    //Get variables from ids
    let data_ptr = get_ptr_from_var_name("data", vm, ids_data, ap_tracking)?;
    let low_addr = get_relocatable_from_var_name("low", vm, ids_data, ap_tracking)?;
    let high_addr = get_relocatable_from_var_name("high", vm, ids_data, ap_tracking)?;
    let mut low = vm.get_integer(low_addr)?.into_owned();
    let mut high = vm.get_integer(high_addr)?.into_owned();
    //Main logic
    //Build first batch of data
    let b = pow2_const_nz(32);
    let mut data = Vec::<MaybeRelocatable>::with_capacity(8);
    for _ in 0..4 {
        let (q, r) = low.div_rem(b);
        data.push(r.into());
        low = q;
    }
    for _ in 0..4 {
        let (q, r) = high.div_rem(b);
        data.push(r.into());
        high = q;
    }
    //Insert second batch of data
    vm.load_data(data_ptr, &data).map_err(HintError::Memory)?;
    Ok(())
}

/* Implements Hint:
    B = 32
    MASK = 2 ** 32 - 1
    segments.write_arg(ids.data, [(ids.high >> (B * (3 - i))) & MASK for i in range(4)])
    segments.write_arg(ids.data + 4, [(ids.low >> (B * (3 - i))) & MASK for i in range(4)])
*/
pub fn blake2s_add_uint256_bigend(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    //Get variables from ids
    let data_ptr = get_ptr_from_var_name("data", vm, ids_data, ap_tracking)?;
    let low_addr = get_relocatable_from_var_name("low", vm, ids_data, ap_tracking)?;
    let high_addr = get_relocatable_from_var_name("high", vm, ids_data, ap_tracking)?;
    let mut low = vm.get_integer(low_addr)?.into_owned();
    let mut high = vm.get_integer(high_addr)?.into_owned();
    //Main logic
    let b = pow2_const_nz(32);
    let mut data = Vec::<MaybeRelocatable>::with_capacity(8);
    //Build first batch of data
    for _ in 0..4 {
        let (q, r) = low.div_rem(b);
        data.push(r.into());
        low = q;
    }
    //Build second batch of data
    for _ in 0..4 {
        let (q, r) = high.div_rem(b);
        data.push(r.into());
        high = q;
    }
    //Reverse to make big-endian
    data.reverse();
    //Insert second batch of data
    vm.load_data(data_ptr, &data).map_err(HintError::Memory)?;
    Ok(())
}

/* Implements Hint:
memory[ap] = (ids.end != ids.packed_values) and (memory[ids.packed_values] < 2**63)
*/
pub fn is_less_than_63_bits_and_not_end(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let end = get_ptr_from_var_name("end", vm, ids_data, ap_tracking)?;
    let packed_values = get_ptr_from_var_name("packed_values", vm, ids_data, ap_tracking)?;

    if end == packed_values {
        insert_value_into_ap(vm, 0)?
    } else {
        let val = vm.get_integer(packed_values)?;
        insert_value_into_ap(
            vm,
            (val.to_biguint() < (BigUint::from(1_u32) << 63)) as usize,
        )?
    }
    Ok(())
}

/* Implements Hint:
offset = 0
for i in range(ids.packed_values_len):
    val = (memory[ids.packed_values + i] % PRIME)
    val_len = 2 if val < 2**63 else 8
    if val_len == 8:
        val += 2**255
    for i in range(val_len - 1, -1, -1):
        val, memory[ids.unpacked_u32s + offset + i] = divmod(val, 2**32)
    assert val == 0
    offset += val_len
*/
pub fn blake2s_unpack_felts(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let packed_values_len =
        get_integer_from_var_name("packed_values_len", vm, ids_data, ap_tracking)?;
    let packed_values = get_ptr_from_var_name("packed_values", vm, ids_data, ap_tracking)?;
    let unpacked_u32s = get_ptr_from_var_name("unpacked_u32s", vm, ids_data, ap_tracking)?;

    let vals = vm.get_integer_range(packed_values, felt_to_usize(&packed_values_len)?)?;
    let pow2_32 = BigUint::from(1_u32) << 32;
    let pow2_63 = BigUint::from(1_u32) << 63;
    let pow2_255 = BigUint::from(1_u32) << 255;

    // Split value into either 2 or 8 32-bit limbs.
    let out: Vec<MaybeRelocatable> = vals
        .into_iter()
        .map(|val| val.to_biguint())
        .flat_map(|val| {
            if val < pow2_63 {
                let (high, low) = val.div_rem(&pow2_32);
                vec![high, low]
            } else {
                let mut limbs = vec![BigUint::from(0_u32); 8];
                let mut val: BigUint = val + &pow2_255;
                for limb in limbs.iter_mut().rev() {
                    let (q, r) = val.div_rem(&pow2_32);
                    *limb = r;
                    val = q;
                }
                limbs
            }
        })
        .map(Felt252::from)
        .map(MaybeRelocatable::from)
        .collect();

    vm.load_data(unpacked_u32s, &out)
        .map_err(HintError::Memory)?;
    Ok(())
}

/* Implements Hint:
    %{
        from starkware.cairo.common.cairo_blake2s.blake2s_utils import IV, blake2s_compress

        _blake2s_input_chunk_size_felts = int(ids.BLAKE2S_INPUT_CHUNK_SIZE_FELTS)
        assert 0 <= _blake2s_input_chunk_size_felts < 100

        new_state = blake2s_compress(
            message=memory.get_range(ids.blake2s_start, _blake2s_input_chunk_size_felts),
            h=[IV[0] ^ 0x01010020] + IV[1:],
            t0=ids.n_bytes,
            t1=0,
            f0=0xffffffff,
            f1=0,
        )

        segments.write_arg(ids.output, new_state)
    %}

Note: This hint belongs to the blake2s lib in cario_examples
*/
pub fn example_blake2s_compress(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let blake2s_start = get_ptr_from_var_name("blake2s_start", vm, ids_data, ap_tracking)?;
    let output = get_ptr_from_var_name("output", vm, ids_data, ap_tracking)?;
    let n_bytes = get_integer_from_var_name("n_bytes", vm, ids_data, ap_tracking).map(|x| {
        x.to_u32()
            .ok_or_else(|| HintError::Math(MathError::Felt252ToU32Conversion(Box::new(x))))
    })??;

    let message = get_fixed_size_u32_array::<16>(&vm.get_integer_range(blake2s_start, 16)?)?;
    let mut modified_iv = IV;
    modified_iv[0] = IV[0] ^ 0x01010020;
    let new_state = blake2s_compress(&modified_iv, &message, n_bytes, 0, 0xffffffff, 0);
    let new_state: Vec<MaybeRelocatable> = new_state
        .iter()
        .map(|x| MaybeRelocatable::from(*x as usize))
        .collect();
    vm.segments.write_arg(output, &new_state)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hint_processor::builtin_hint_processor::hint_code;
    use crate::types::errors::math_errors::MathError;
    use crate::{
        any_box,
        hint_processor::{
            builtin_hint_processor::builtin_hint_processor_definition::{
                BuiltinHintProcessor, HintProcessorData,
            },
            hint_processor_definition::HintProcessorLogic,
        },
        relocatable,
        utils::test_utils::*,
        vm::errors::memory_errors::MemoryError,
    };
    use assert_matches::assert_matches;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_blake2s_output_offset_zero() {
        let hint_code = "from starkware.cairo.common.cairo_blake2s.blake2s_utils import compute_blake2s_func\ncompute_blake2s_func(segments=segments, output_ptr=ids.output)";
        //Create vm
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 1;
        //Insert ids into memory (output)
        vm.segments = segments![((1, 0), (2, 5))];
        //Create hint data
        let ids_data = ids_data!["output"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Math(MathError::RelocatableSubUsizeNegOffset(bx)))
            if *bx == (relocatable!(2,5), 26)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_blake2s_output_empty_segment() {
        let hint_code = "from starkware.cairo.common.cairo_blake2s.blake2s_utils import compute_blake2s_func\ncompute_blake2s_func(segments=segments, output_ptr=ids.output)";
        //Create vm
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 1;
        //Insert ids into memory (output)
        vm.segments = segments![((1, 0), (2, 26))];
        add_segments!(vm, 1);
        //Create hint data
        let ids_data = ids_data!["output"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(MemoryError::UnknownMemoryCell(
                bx
            ))) if *bx == Relocatable::from((2, 0))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_blake2s_output_not_relocatable() {
        let hint_code = "from starkware.cairo.common.cairo_blake2s.blake2s_utils import compute_blake2s_func\ncompute_blake2s_func(segments=segments, output_ptr=ids.output)";
        //Create vm
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 1;
        //Insert ids into memory (output)
        vm.segments = segments![((1, 0), 12)];
        //Create hint data
        let ids_data = ids_data!["output"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::IdentifierNotRelocatable(bx))
            if bx.as_ref() == "output"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_blake2s_output_input_bigger_than_u32() {
        let hint_code = "from starkware.cairo.common.cairo_blake2s.blake2s_utils import compute_blake2s_func\ncompute_blake2s_func(segments=segments, output_ptr=ids.output)";
        //Create vm
        let mut vm = vm!();
        //Initialize fp
        //Insert ids into memory
        vm.run_context.fp = 1;
        vm.segments = segments![
            ((1, 0), (2, 26)),
            ((2, 0), 7842562439562793675803603603688959_i128),
            ((2, 1), 7842562439562793675803603603688959_i128),
            ((2, 2), 7842562439562793675803603603688959_i128),
            ((2, 3), 7842562439562793675803603603688959_i128),
            ((2, 4), 7842562439562793675803603603688959_i128),
            ((2, 5), 7842562439562793675803603603688959_i128),
            ((2, 6), 7842562439562793675803603603688959_i128),
            ((2, 7), 7842562439562793675803603603688959_i128)
        ];
        //Create hint data
        let ids_data = ids_data!["output"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::BigintToU32Fail)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_blake2s_output_input_relocatable() {
        let hint_code = "from starkware.cairo.common.cairo_blake2s.blake2s_utils import compute_blake2s_func\ncompute_blake2s_func(segments=segments, output_ptr=ids.output)";
        //Create vm
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 1;
        //Insert ids into memory (output)
        vm.segments = segments![((1, 0), (2, 26)), ((2, 0), (5, 5))];
        //Create hint data
        let ids_data = ids_data!["output"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(MemoryError::ExpectedInteger(
                bx
            ))) if *bx == Relocatable::from((2, 0))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn finalize_blake2s_valid() {
        let hint_code = "# Add dummy pairs of input and output.\nfrom starkware.cairo.common.cairo_blake2s.blake2s_utils import IV, blake2s_compress\n\n_n_packed_instances = int(ids.N_PACKED_INSTANCES)\nassert 0 <= _n_packed_instances < 20\n_blake2s_input_chunk_size_felts = int(ids.INPUT_BLOCK_FELTS)\nassert 0 <= _blake2s_input_chunk_size_felts < 100\n\nmessage = [0] * _blake2s_input_chunk_size_felts\nmodified_iv = [IV[0] ^ 0x01010020] + IV[1:]\noutput = blake2s_compress(\n    message=message,\n    h=modified_iv,\n    t0=0,\n    t1=0,\n    f0=0xffffffff,\n    f1=0,\n)\npadding = (modified_iv + message + [0, 0xffffffff] + output) * (_n_packed_instances - 1)\nsegments.write_arg(ids.blake2s_ptr_end, padding)";
        //Create vm
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 1;
        //Insert ids into memory (output)
        vm.segments = segments![((1, 0), (2, 0))];
        add_segments!(vm, 1);
        //Create hint data
        let ids_data = ids_data!["blake2s_ptr_end"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check the inserted data
        let expected_data: [u32; 204] = [
            1795745351, 3144134277, 1013904242, 2773480762, 1359893119, 2600822924, 528734635,
            1541459225, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4294967295, 813310313,
            2491453561, 3491828193, 2085238082, 1219908895, 514171180, 4245497115, 4193177630,
            1795745351, 3144134277, 1013904242, 2773480762, 1359893119, 2600822924, 528734635,
            1541459225, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4294967295, 813310313,
            2491453561, 3491828193, 2085238082, 1219908895, 514171180, 4245497115, 4193177630,
            1795745351, 3144134277, 1013904242, 2773480762, 1359893119, 2600822924, 528734635,
            1541459225, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4294967295, 813310313,
            2491453561, 3491828193, 2085238082, 1219908895, 514171180, 4245497115, 4193177630,
            1795745351, 3144134277, 1013904242, 2773480762, 1359893119, 2600822924, 528734635,
            1541459225, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4294967295, 813310313,
            2491453561, 3491828193, 2085238082, 1219908895, 514171180, 4245497115, 4193177630,
            1795745351, 3144134277, 1013904242, 2773480762, 1359893119, 2600822924, 528734635,
            1541459225, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4294967295, 813310313,
            2491453561, 3491828193, 2085238082, 1219908895, 514171180, 4245497115, 4193177630,
            1795745351, 3144134277, 1013904242, 2773480762, 1359893119, 2600822924, 528734635,
            1541459225, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4294967295, 813310313,
            2491453561, 3491828193, 2085238082, 1219908895, 514171180, 4245497115, 4193177630,
        ];
        //Get data from memory
        let data = get_fixed_size_u32_array::<204>(
            &vm.segments
                .memory
                .get_integer_range(relocatable!(2, 0), 204)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(expected_data, data);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn finalize_blake2s_invalid_segment_taken() {
        let hint_code = "# Add dummy pairs of input and output.\nfrom starkware.cairo.common.cairo_blake2s.blake2s_utils import IV, blake2s_compress\n\n_n_packed_instances = int(ids.N_PACKED_INSTANCES)\nassert 0 <= _n_packed_instances < 20\n_blake2s_input_chunk_size_felts = int(ids.INPUT_BLOCK_FELTS)\nassert 0 <= _blake2s_input_chunk_size_felts < 100\n\nmessage = [0] * _blake2s_input_chunk_size_felts\nmodified_iv = [IV[0] ^ 0x01010020] + IV[1:]\noutput = blake2s_compress(\n    message=message,\n    h=modified_iv,\n    t0=0,\n    t1=0,\n    f0=0xffffffff,\n    f1=0,\n)\npadding = (modified_iv + message + [0, 0xffffffff] + output) * (_n_packed_instances - 1)\nsegments.write_arg(ids.blake2s_ptr_end, padding)";
        //Create vm
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 1;
        //Insert ids into memory (output)
        vm.segments = segments![((1, 0), (2, 0)), ((2, 0), (2, 0))];
        let ids_data = ids_data!["blake2s_ptr_end"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(
                MemoryError::InconsistentMemory(bx)
            )) if *bx == (Relocatable::from((2, 0)),
                    MaybeRelocatable::from((2, 0)),
                    MaybeRelocatable::from(Felt252::from(1795745351)))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn finalize_blake2s_invalid_no_ids() {
        let hint_code = "# Add dummy pairs of input and output.\nfrom starkware.cairo.common.cairo_blake2s.blake2s_utils import IV, blake2s_compress\n\n_n_packed_instances = int(ids.N_PACKED_INSTANCES)\nassert 0 <= _n_packed_instances < 20\n_blake2s_input_chunk_size_felts = int(ids.INPUT_BLOCK_FELTS)\nassert 0 <= _blake2s_input_chunk_size_felts < 100\n\nmessage = [0] * _blake2s_input_chunk_size_felts\nmodified_iv = [IV[0] ^ 0x01010020] + IV[1:]\noutput = blake2s_compress(\n    message=message,\n    h=modified_iv,\n    t0=0,\n    t1=0,\n    f0=0xffffffff,\n    f1=0,\n)\npadding = (modified_iv + message + [0, 0xffffffff] + output) * (_n_packed_instances - 1)\nsegments.write_arg(ids.blake2s_ptr_end, padding)";
        //Create vm
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 1;
        //Execute the hint
        assert_matches!(
            run_hint!(vm, HashMap::new(), hint_code),
            Err(HintError::UnknownIdentifier(bx)) if bx.as_ref() == "blake2s_ptr_end"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn blake2s_add_uint256_valid_zero() {
        let hint_code = "B = 32\nMASK = 2 ** 32 - 1\nsegments.write_arg(ids.data, [(ids.low >> (B * i)) & MASK for i in range(4)])\nsegments.write_arg(ids.data + 4, [(ids.high >> (B * i)) & MASK for i in range(4)])";
        //Create vm
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 3;
        //Insert ids into memory
        vm.segments = segments![((1, 0), (2, 0)), ((1, 1), 0), ((1, 2), 0)];
        vm.segments.add();
        let ids_data = ids_data!["data", "high", "low"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check data ptr
        check_memory![
            vm.segments.memory,
            ((2, 0), 0),
            ((2, 1), 0),
            ((2, 2), 0),
            ((2, 3), 0),
            ((2, 4), 0),
            ((2, 5), 0),
            ((2, 6), 0),
            ((2, 7), 0)
        ];
        assert!(vm
            .segments
            .memory
            .get(&MaybeRelocatable::from((2, 8)))
            .is_none());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn blake2s_add_uint256_valid_non_zero() {
        let hint_code = "B = 32\nMASK = 2 ** 32 - 1\nsegments.write_arg(ids.data, [(ids.low >> (B * i)) & MASK for i in range(4)])\nsegments.write_arg(ids.data + 4, [(ids.high >> (B * i)) & MASK for i in range(4)])";
        //Create vm
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 3;
        //Insert ids into memory
        vm.segments = segments![((1, 0), (2, 0)), ((1, 1), 25), ((1, 2), 20)];
        vm.segments.add();
        let ids_data = ids_data!["data", "high", "low"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check data ptr
        check_memory![
            vm.segments.memory,
            ((2, 0), 20),
            ((2, 1), 0),
            ((2, 2), 0),
            ((2, 3), 0),
            ((2, 4), 25),
            ((2, 5), 0),
            ((2, 6), 0),
            ((2, 7), 0)
        ];
        assert!(vm
            .segments
            .memory
            .get(&MaybeRelocatable::from((2, 8)))
            .is_none());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn blake2s_add_uint256_bigend_valid_zero() {
        let hint_code = "B = 32\nMASK = 2 ** 32 - 1\nsegments.write_arg(ids.data, [(ids.high >> (B * (3 - i))) & MASK for i in range(4)])\nsegments.write_arg(ids.data + 4, [(ids.low >> (B * (3 - i))) & MASK for i in range(4)])";
        //Create vm
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 3;
        //Insert ids into memory
        vm.segments = segments![((1, 0), (2, 0)), ((1, 1), 0), ((1, 2), 0)];
        add_segments!(vm, 1);
        let ids_data = ids_data!["data", "high", "low"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check data ptr
        check_memory![
            vm.segments.memory,
            ((2, 0), 0),
            ((2, 1), 0),
            ((2, 2), 0),
            ((2, 3), 0),
            ((2, 4), 0),
            ((2, 5), 0),
            ((2, 6), 0),
            ((2, 7), 0)
        ];
        assert!(vm
            .segments
            .memory
            .get(&MaybeRelocatable::from((2, 8)))
            .is_none());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn is_less_than_63_bits_and_not_end_ends() {
        let hint_code = hint_code::IS_LESS_THAN_63_BITS_AND_NOT_END;
        //Create vm
        let mut vm = vm!();
        //Insert ids into memory
        vm.segments = segments![((1, 0), (1, 2)), ((1, 1), (1, 2)), ((1, 2), 123)];
        vm.set_fp(3);
        vm.set_ap(3);
        let ids_data = ids_data!["end", "packed_values", "value"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check data ptr
        check_memory![vm.segments.memory, ((1, 3), 0)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn is_less_than_63_bits_and_not_end_small() {
        let hint_code = hint_code::IS_LESS_THAN_63_BITS_AND_NOT_END;
        //Create vm
        let mut vm = vm!();
        //Insert ids into memory
        vm.segments = segments![((1, 0), (1, 3)), ((1, 1), (1, 2)), ((1, 2), 123)];
        vm.set_fp(3);
        vm.set_ap(3);
        let ids_data = ids_data!["end", "packed_values", "value"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check data ptr
        check_memory![vm.segments.memory, ((1, 3), 1)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn is_less_than_63_bits_and_not_end_big() {
        let hint_code = hint_code::IS_LESS_THAN_63_BITS_AND_NOT_END;
        //Create vm
        let mut vm = vm!();
        //Insert ids into memory
        vm.segments = segments![
            ((1, 0), (1, 3)),
            ((1, 1), (1, 2)),
            ((1, 2), 0x10000000000000000)
        ];
        vm.set_fp(3);
        vm.set_ap(3);
        let ids_data = ids_data!["end", "packed_values", "value"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check data ptr
        check_memory![vm.segments.memory, ((1, 3), 0)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn blake2s_unpack_felts() {
        let hint_code = hint_code::BLAKE2S_UNPACK_FELTS;
        //Create vm
        let mut vm = vm!();
        //Insert ids into memory
        vm.segments = segments![
            ((1, 0), 2),
            ((1, 1), (1, 3)),
            ((1, 2), (2, 0)),
            ((1, 3), 0x123456781234),
            ((1, 4), 0x1234abcd5678efab1234abcd)
        ];
        vm.set_fp(5);
        vm.set_ap(5);
        let ids_data = ids_data![
            "packed_values_len",
            "packed_values",
            "unpacked_u32s",
            "small_value",
            "big_value"
        ];
        vm.segments.add();
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check data ptr
        check_memory![
            vm.segments.memory,
            ((2, 0), 0x1234),
            ((2, 1), 0x56781234),
            ((2, 2), 0x80000000),
            ((2, 3), 0),
            ((2, 4), 0),
            ((2, 5), 0),
            ((2, 6), 0),
            ((2, 7), 0x1234abcd),
            ((2, 8), 0x5678efab),
            ((2, 9), 0x1234abcd)
        ];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn blake2s_add_uint256_bigend_valid_non_zero() {
        let hint_code = "B = 32\nMASK = 2 ** 32 - 1\nsegments.write_arg(ids.data, [(ids.high >> (B * (3 - i))) & MASK for i in range(4)])\nsegments.write_arg(ids.data + 4, [(ids.low >> (B * (3 - i))) & MASK for i in range(4)])";
        //Create vm
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 3;
        //Insert ids into memory
        vm.segments = segments![((1, 0), (2, 0)), ((1, 1), 25), ((1, 2), 20)];
        vm.segments.add();
        let ids_data = ids_data!["data", "high", "low"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check data ptr
        check_memory![
            vm.segments.memory,
            ((2, 0), 0),
            ((2, 1), 0),
            ((2, 2), 0),
            ((2, 3), 25),
            ((2, 4), 0),
            ((2, 5), 0),
            ((2, 6), 0),
            ((2, 7), 20)
        ];
        assert!(vm
            .segments
            .memory
            .get(&MaybeRelocatable::from((2, 8)))
            .is_none());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn example_blake2s_compress_n_bytes_over_u32() {
        //Create vm
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 3;
        //Insert ids into memory
        vm.segments = segments![((1, 0), 9999999999_u64), ((1, 1), (1, 0)), ((1, 2), (2, 0))];
        let ids_data = ids_data!["n_bytes", "output", "blake2s_start"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code::EXAMPLE_BLAKE2S_COMPRESS), Err(HintError::Math(MathError::Felt252ToU32Conversion(bx))) if *bx == Felt252::from(9999999999_u64));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn example_blake2s_empty_input() {
        //Create vm
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 3;
        //Insert ids into memory
        vm.segments = segments![((1, 0), 9), ((1, 1), (1, 0)), ((1, 2), (2, 0))];
        let ids_data = ids_data!["n_bytes", "output", "blake2s_start"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code::EXAMPLE_BLAKE2S_COMPRESS), Err(HintError::Memory(MemoryError::UnknownMemoryCell(bx))) if *bx == (2, 0).into());
    }
}
