use crate::Felt252;
use num_bigint::{BigUint, ToBigInt};
use num_integer::Integer;
use num_traits::Zero;

use super::hint_utils::insert_value_from_var_name;
use super::secp::bigint_utils::Uint384;
use super::uint256_utils::Uint256;
use crate::math_utils::{is_quad_residue, mul_inv, sqrt_prime_power};
use crate::serde::deserialize_program::ApTracking;
use crate::stdlib::{collections::HashMap, prelude::*};
use crate::types::errors::math_errors::MathError;
use crate::vm::errors::hint_errors::HintError;
use crate::{
    hint_processor::hint_processor_definition::HintReference, vm::vm_core::VirtualMachine,
};

/* Implements Hint:
%{
    from starkware.python.math_utils import is_quad_residue, sqrt

    def split(num: int, num_bits_shift: int = 128, length: int = 3):
        a = []
        for _ in range(length):
            a.append( num & ((1 << num_bits_shift) - 1) )
            num = num >> num_bits_shift
        return tuple(a)

    def pack(z, num_bits_shift: int = 128) -> int:
        limbs = (z.d0, z.d1, z.d2)
        return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))


    generator = pack(ids.generator)
    x = pack(ids.x)
    p = pack(ids.p)

    success_x = is_quad_residue(x, p)
    root_x = sqrt(x, p) if success_x else None

    success_gx = is_quad_residue(generator*x, p)
    root_gx = sqrt(generator*x, p) if success_gx else None

    # Check that one is 0 and the other is 1
    if x != 0:
        assert success_x + success_gx ==1

    # `None` means that no root was found, but we need to transform these into a felt no matter what
    if root_x == None:
        root_x = 0
    if root_gx == None:
        root_gx = 0
    ids.success_x = int(success_x)
    ids.success_gx = int(success_gx)
    split_root_x = split(root_x)
    split_root_gx = split(root_gx)
    ids.sqrt_x.d0 = split_root_x[0]
    ids.sqrt_x.d1 = split_root_x[1]
    ids.sqrt_x.d2 = split_root_x[2]
    ids.sqrt_gx.d0 = split_root_gx[0]
    ids.sqrt_gx.d1 = split_root_gx[1]
    ids.sqrt_gx.d2 = split_root_gx[2]
%}
*/
pub fn u384_get_square_root(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let generator = Uint384::from_var_name("generator", vm, ids_data, ap_tracking)?.pack();
    let x = Uint384::from_var_name("x", vm, ids_data, ap_tracking)?.pack();
    let p = Uint384::from_var_name("p", vm, ids_data, ap_tracking)?.pack();
    let success_x = is_quad_residue(&x, &p)?;

    let root_x = if success_x {
        sqrt_prime_power(&x, &p).unwrap_or_default()
    } else {
        BigUint::zero()
    };

    let gx = generator * &x;
    let success_gx = is_quad_residue(&gx, &p)?;

    let root_gx = if success_gx {
        sqrt_prime_power(&gx, &p).unwrap_or_default()
    } else {
        BigUint::zero()
    };

    if !&x.is_zero() && !(success_x ^ success_gx) {
        return Err(HintError::AssertionFailed(
            "assert success_x + success_gx ==1"
                .to_string()
                .into_boxed_str(),
        ));
    }
    insert_value_from_var_name(
        "success_x",
        Felt252::from(success_x as u8),
        vm,
        ids_data,
        ap_tracking,
    )?;
    insert_value_from_var_name(
        "success_gx",
        Felt252::from(success_gx as u8),
        vm,
        ids_data,
        ap_tracking,
    )?;
    Uint384::split(&root_x).insert_from_var_name("sqrt_x", vm, ids_data, ap_tracking)?;
    Uint384::split(&root_gx).insert_from_var_name("sqrt_gx", vm, ids_data, ap_tracking)?;
    Ok(())
}

/* Implements Hint:
%{
    from starkware.python.math_utils import is_quad_residue, sqrt

    def split(a: int):
        return (a & ((1 << 128) - 1), a >> 128)

    def pack(z) -> int:
        return z.low + (z.high << 128)

    generator = pack(ids.generator)
    x = pack(ids.x)
    p = pack(ids.p)

    success_x = is_quad_residue(x, p)
    root_x = sqrt(x, p) if success_x else None
    success_gx = is_quad_residue(generator*x, p)
    root_gx = sqrt(generator*x, p) if success_gx else None

    # Check that one is 0 and the other is 1
    if x != 0:
        assert success_x + success_gx == 1

    # `None` means that no root was found, but we need to transform these into a felt no matter what
    if root_x == None:
        root_x = 0
    if root_gx == None:
        root_gx = 0
    ids.success_x = int(success_x)
    ids.success_gx = int(success_gx)
    split_root_x = split(root_x)
    # print('split root x', split_root_x)
    split_root_gx = split(root_gx)
    ids.sqrt_x.low = split_root_x[0]
    ids.sqrt_x.high = split_root_x[1]
    ids.sqrt_gx.low = split_root_gx[0]
    ids.sqrt_gx.high = split_root_gx[1]
%}
*/
// TODO: extract UintNNN methods to a trait, and use generics
//  to merge this with u384_get_square_root
pub fn u256_get_square_root(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let generator = Uint256::from_var_name("generator", vm, ids_data, ap_tracking)?.pack();
    let x = Uint256::from_var_name("x", vm, ids_data, ap_tracking)?.pack();
    let p = Uint256::from_var_name("p", vm, ids_data, ap_tracking)?.pack();
    let success_x = is_quad_residue(&x, &p)?;

    let root_x = if success_x {
        sqrt_prime_power(&x, &p).unwrap_or_default()
    } else {
        BigUint::zero()
    };

    let gx = generator * &x;
    let success_gx = is_quad_residue(&gx, &p)?;

    let root_gx = if success_gx {
        sqrt_prime_power(&gx, &p).unwrap_or_default()
    } else {
        BigUint::zero()
    };

    if !&x.is_zero() && !(success_x ^ success_gx) {
        return Err(HintError::AssertionFailed(
            "assert success_x + success_gx ==1"
                .to_string()
                .into_boxed_str(),
        ));
    }
    insert_value_from_var_name(
        "success_x",
        Felt252::from(success_x as u8),
        vm,
        ids_data,
        ap_tracking,
    )?;
    insert_value_from_var_name(
        "success_gx",
        Felt252::from(success_gx as u8),
        vm,
        ids_data,
        ap_tracking,
    )?;
    Uint256::split(&root_x).insert_from_var_name("sqrt_x", vm, ids_data, ap_tracking)?;
    Uint256::split(&root_gx).insert_from_var_name("sqrt_gx", vm, ids_data, ap_tracking)?;
    Ok(())
}

/* Implements Hint:
 %{
    from starkware.python.math_utils import div_mod

    def split(num: int, num_bits_shift: int, length: int):
        a = []
        for _ in range(length):
            a.append( num & ((1 << num_bits_shift) - 1) )
            num = num >> num_bits_shift
        return tuple(a)

    def pack(z, num_bits_shift: int) -> int:
        limbs = (z.d0, z.d1, z.d2)
        return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

    a = pack(ids.a, num_bits_shift = 128)
    b = pack(ids.b, num_bits_shift = 128)
    p = pack(ids.p, num_bits_shift = 128)
    # For python3.8 and above the modular inverse can be computed as follows:
    # b_inverse_mod_p = pow(b, -1, p)
    # Instead we use the python3.7-friendly function div_mod from starkware.python.math_utils
    b_inverse_mod_p = div_mod(1, b, p)


    b_inverse_mod_p_split = split(b_inverse_mod_p, num_bits_shift=128, length=3)

    ids.b_inverse_mod_p.d0 = b_inverse_mod_p_split[0]
    ids.b_inverse_mod_p.d1 = b_inverse_mod_p_split[1]
    ids.b_inverse_mod_p.d2 = b_inverse_mod_p_split[2]
%}
 */
pub fn uint384_div(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    // Note: ids.a is not used here, nor is it used by following hints, so we dont need to extract it.
    let b = Uint384::from_var_name("b", vm, ids_data, ap_tracking)?
        .pack()
        .to_bigint()
        .unwrap_or_default();
    let p = Uint384::from_var_name("p", vm, ids_data, ap_tracking)?
        .pack()
        .to_bigint()
        .unwrap_or_default();

    if b.is_zero() {
        return Err(MathError::DividedByZero.into());
    }
    let b_inverse_mod_p = mul_inv(&b, &p)
        .mod_floor(&p)
        .to_biguint()
        .unwrap_or_default();
    let b_inverse_mod_p_split = Uint384::split(&b_inverse_mod_p);
    b_inverse_mod_p_split.insert_from_var_name("b_inverse_mod_p", vm, ids_data, ap_tracking)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hint_processor::builtin_hint_processor::hint_code;
    use crate::vm::errors::memory_errors::MemoryError;
    use crate::{
        any_box,
        hint_processor::{
            builtin_hint_processor::builtin_hint_processor_definition::{
                BuiltinHintProcessor, HintProcessorData,
            },
            hint_processor_definition::HintProcessorLogic,
        },
        utils::test_utils::*,
        vm::vm_core::VirtualMachine,
    };
    use assert_matches::assert_matches;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_u384_get_square_ok_goldilocks_prime() {
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 14;
        //Create hint_data
        let ids_data = non_continuous_ids_data![
            ("p", -14),
            ("x", -11),
            ("generator", -8),
            ("sqrt_x", -5),
            ("sqrt_gx", -2),
            ("success_x", 1),
            ("success_gx", 2)
        ];
        //Insert ids into memory
        vm.segments = segments![
            //p
            ((1, 0), 18446744069414584321),
            ((1, 1), 0),
            ((1, 2), 0),
            //x
            ((1, 3), 25),
            ((1, 4), 0),
            ((1, 5), 0),
            //generator
            ((1, 6), 7),
            ((1, 7), 0),
            ((1, 8), 0)
        ];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::UINT384_GET_SQUARE_ROOT),
            Ok(())
        );
        //Check hint memory inserts
        check_memory![
            vm.segments.memory,
            // sqrt_x
            ((1, 9), 5),
            ((1, 10), 0),
            ((1, 11), 0),
            // sqrt_gx
            ((1, 12), 0),
            ((1, 13), 0),
            ((1, 14), 0),
            // success_x
            ((1, 15), 1),
            // success_gx
            ((1, 16), 0)
        ];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_u384_get_square_no_successes() {
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 14;
        //Create hint_data
        let ids_data = non_continuous_ids_data![
            ("p", -14),
            ("x", -11),
            ("generator", -8),
            ("sqrt_x", -5),
            ("sqrt_gx", -2),
            ("success_x", 1),
            ("success_gx", 2)
        ];
        //Insert ids into memory
        vm.segments = segments![
            //p
            ((1, 0), 3),
            ((1, 1), 0),
            ((1, 2), 0),
            //x
            ((1, 3), 17),
            ((1, 4), 0),
            ((1, 5), 0),
            //generator
            ((1, 6), 1),
            ((1, 7), 0),
            ((1, 8), 0)
        ];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code::UINT384_GET_SQUARE_ROOT),
            Err(HintError::AssertionFailed(bx)) if bx.as_ref() == "assert success_x + success_gx ==1"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_u384_get_square_ok_success_gx() {
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 14;
        //Create hint_data
        let ids_data = non_continuous_ids_data![
            ("p", -14),
            ("x", -11),
            ("generator", -8),
            ("sqrt_x", -5),
            ("sqrt_gx", -2),
            ("success_x", 1),
            ("success_gx", 2),
        ];
        //Insert ids into memory
        vm.segments = segments![
            //p
            ((1, 0), 3),
            ((1, 1), 0),
            ((1, 2), 0),
            //x
            ((1, 3), 17),
            ((1, 4), 0),
            ((1, 5), 0),
            //generator
            ((1, 6), 71),
            ((1, 7), 0),
            ((1, 8), 0),
        ];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::UINT384_GET_SQUARE_ROOT),
            Ok(())
        );
        //Check hint memory inserts
        check_memory![
            vm.segments.memory,
            // sqrt_x
            ((1, 9), 0),
            ((1, 10), 0),
            ((1, 11), 0),
            // sqrt_gx
            ((1, 12), 1),
            ((1, 13), 0),
            ((1, 14), 0),
            // success_x
            ((1, 15), 0),
            // success_gx
            ((1, 16), 1),
        ];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_u256_get_square_ok_goldilocks_prime() {
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 14;
        //Create hint_data
        let ids_data = non_continuous_ids_data![
            ("p", -14),
            ("x", -11),
            ("generator", -8),
            ("sqrt_x", -5),
            ("sqrt_gx", -2),
            ("success_x", 1),
            ("success_gx", 2),
        ];
        //Insert ids into memory
        vm.segments = segments![
            //p
            ((1, 0), 18446744069414584321),
            ((1, 1), 0),
            //x
            ((1, 3), 25),
            ((1, 4), 0),
            //generator
            ((1, 6), 7),
            ((1, 7), 0),
        ];
        //Execute the hint
        assert!(run_hint!(vm, ids_data, hint_code::UINT256_GET_SQUARE_ROOT).is_ok());
        //Check hint memory inserts
        check_memory![
            vm.segments.memory,
            // sqrt_x
            ((1, 9), 5),
            ((1, 10), 0),
            // sqrt_gx
            ((1, 12), 0),
            ((1, 13), 0),
            // success_x
            ((1, 15), 1),
            // success_gx
            ((1, 16), 0),
        ];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_u256_get_square_no_successes() {
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 14;
        //Create hint_data
        let ids_data = non_continuous_ids_data![
            ("p", -14),
            ("x", -11),
            ("generator", -8),
            ("sqrt_x", -5),
            ("sqrt_gx", -2),
            ("success_x", 1),
            ("success_gx", 2),
        ];
        //Insert ids into memory
        vm.segments = segments![
            //p
            ((1, 0), 3),
            ((1, 1), 0),
            //x
            ((1, 3), 17),
            ((1, 4), 0),
            //generator
            ((1, 6), 1),
            ((1, 7), 0),
        ];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code::UINT256_GET_SQUARE_ROOT),
            Err(HintError::AssertionFailed(bx)) if bx.as_ref() == "assert success_x + success_gx ==1"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_u256_get_square_ok_success_gx() {
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 14;
        //Create hint_data
        let ids_data = non_continuous_ids_data![
            ("p", -14),
            ("x", -11),
            ("generator", -8),
            ("sqrt_x", -5),
            ("sqrt_gx", -2),
            ("success_x", 1),
            ("success_gx", 2),
        ];
        //Insert ids into memory
        vm.segments = segments![
            //p
            ((1, 0), 3),
            ((1, 1), 0),
            //x
            ((1, 3), 17),
            ((1, 4), 0),
            //generator
            ((1, 6), 71),
            ((1, 7), 0),
        ];
        //Execute the hint
        assert!(run_hint!(vm, ids_data, hint_code::UINT256_GET_SQUARE_ROOT).is_ok());
        //Check hint memory inserts
        check_memory![
            vm.segments.memory,
            // sqrt_x
            ((1, 9), 0),
            ((1, 10), 0),
            // sqrt_gx
            ((1, 12), 1),
            ((1, 13), 0),
            // success_x
            ((1, 15), 0),
            // success_gx
            ((1, 16), 1),
        ];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_uint384_div_ok() {
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 11;
        //Create hint_data
        let ids_data =
            non_continuous_ids_data![("a", -11), ("b", -8), ("p", -5), ("b_inverse_mod_p", -2)];
        //Insert ids into memory
        vm.segments = segments![
            //a
            ((1, 0), 25),
            ((1, 1), 0),
            ((1, 2), 0),
            //b
            ((1, 3), 5),
            ((1, 4), 0),
            ((1, 5), 0),
            //p
            ((1, 6), 31),
            ((1, 7), 0),
            ((1, 8), 0)
        ];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code::UINT384_DIV), Ok(()));
        //Check hint memory inserts
        check_memory![
            vm.segments.memory,
            // b_inverse_mod_p
            ((1, 9), 25),
            ((1, 10), 0),
            ((1, 11), 0)
        ];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_uint384_div_b_is_zero() {
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 11;
        //Create hint_data
        let ids_data =
            non_continuous_ids_data![("a", -11), ("b", -8), ("p", -5), ("b_inverse_mod_p", -2)];
        //Insert ids into memory
        vm.segments = segments![
            //a
            ((1, 0), 25),
            ((1, 1), 0),
            ((1, 2), 0),
            //b
            ((1, 3), 0),
            ((1, 4), 0),
            ((1, 5), 0),
            //p
            ((1, 6), 31),
            ((1, 7), 0),
            ((1, 8), 0)
        ];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::UINT384_DIV),
            Err(HintError::Math(MathError::DividedByZero))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_uint384_div_inconsistent_memory() {
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 11;
        //Create hint_data
        let ids_data =
            non_continuous_ids_data![("a", -11), ("b", -8), ("p", -5), ("b_inverse_mod_p", -2)];
        //Insert ids into memory
        vm.segments = segments![
            //a
            ((1, 0), 25),
            ((1, 1), 0),
            ((1, 2), 0),
            //b
            ((1, 3), 5),
            ((1, 4), 0),
            ((1, 5), 0),
            //p
            ((1, 6), 31),
            ((1, 7), 0),
            ((1, 8), 0),
            //b_inverse_mod_p
            ((1, 9), 0)
        ];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::UINT384_DIV),
            Err(HintError::Memory(MemoryError::InconsistentMemory(_)))
        );
    }
}
