use crate::Felt252;
use crate::{
    hint_processor::builtin_hint_processor::hint_utils::{
        get_integer_from_var_name, get_relocatable_from_var_name, insert_value_from_var_name,
        insert_value_into_ap,
    },
    hint_processor::hint_processor_definition::HintReference,
    math_utils::{isqrt, pow2_const, pow2_const_nz},
    serde::deserialize_program::ApTracking,
    stdlib::{
        borrow::Cow,
        boxed::Box,
        collections::HashMap,
        ops::{Shl, Shr},
        prelude::*,
    },
    types::{errors::math_errors::MathError, relocatable::Relocatable},
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};
use num_bigint::BigUint;
use num_integer::{div_rem, Integer};
use num_traits::{One, Zero};

// TODO: use this type in all uint256 functions
pub(crate) struct Uint256<'a> {
    pub low: Cow<'a, Felt252>,
    pub high: Cow<'a, Felt252>,
}

impl<'a> Uint256<'a> {
    pub(crate) fn from_base_addr(
        addr: Relocatable,
        name: &str,
        vm: &'a VirtualMachine,
    ) -> Result<Self, HintError> {
        Ok(Self {
            low: vm.get_integer(addr).map_err(|_| {
                HintError::IdentifierHasNoMember(Box::new((name.to_string(), "low".to_string())))
            })?,
            high: vm.get_integer((addr + 1)?).map_err(|_| {
                HintError::IdentifierHasNoMember(Box::new((name.to_string(), "high".to_string())))
            })?,
        })
    }

    pub(crate) fn from_var_name(
        name: &str,
        vm: &'a VirtualMachine,
        ids_data: &HashMap<String, HintReference>,
        ap_tracking: &ApTracking,
    ) -> Result<Self, HintError> {
        let base_addr = get_relocatable_from_var_name(name, vm, ids_data, ap_tracking)?;
        Self::from_base_addr(base_addr, name, vm)
    }

    pub(crate) fn from_values(low: Felt252, high: Felt252) -> Self {
        let low = Cow::Owned(low);
        let high = Cow::Owned(high);
        Self { low, high }
    }

    pub(crate) fn insert_from_var_name(
        self,
        var_name: &str,
        vm: &mut VirtualMachine,
        ids_data: &HashMap<String, HintReference>,
        ap_tracking: &ApTracking,
    ) -> Result<(), HintError> {
        let addr = get_relocatable_from_var_name(var_name, vm, ids_data, ap_tracking)?;

        vm.insert_value(addr, self.low.into_owned())?;
        vm.insert_value((addr + 1)?, self.high.into_owned())?;

        Ok(())
    }

    pub(crate) fn pack(self) -> BigUint {
        (self.high.to_biguint() << 128) + self.low.to_biguint()
    }

    pub(crate) fn split(num: &BigUint) -> Self {
        let mask_low: BigUint = u128::MAX.into();
        let low = Felt252::from(&(num & mask_low));
        let high = Felt252::from(&(num >> 128));
        Self::from_values(low, high)
    }
}

impl From<&BigUint> for Uint256<'_> {
    fn from(value: &BigUint) -> Self {
        Self::split(value)
    }
}

impl From<Felt252> for Uint256<'_> {
    fn from(value: Felt252) -> Self {
        let (high, low) = value.div_rem(pow2_const_nz(128));
        Self::from_values(low, high)
    }
}

/*
Implements hints:
%{
    sum_low = ids.a.low + ids.b.low
    ids.carry_low = 1 if sum_low >= ids.SHIFT else 0
    sum_high = ids.a.high + ids.b.high + ids.carry_low
    ids.carry_high = 1 if sum_high >= ids.SHIFT else 0
%}
%{
    sum_low = ids.a.low + ids.b.low
    ids.carry_low = 1 if sum_low >= ids.SHIFT else 0
%}
*/
pub fn uint256_add(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    low_only: bool,
) -> Result<(), HintError> {
    let shift = pow2_const(128);

    let a = Uint256::from_var_name("a", vm, ids_data, ap_tracking)?;
    let b = Uint256::from_var_name("b", vm, ids_data, ap_tracking)?;
    let a_low = a.low.as_ref();
    let b_low = b.low.as_ref();

    // Main logic
    // sum_low = ids.a.low + ids.b.low
    // ids.carry_low = 1 if sum_low >= ids.SHIFT else 0
    let carry_low = Felt252::from((a_low + b_low >= shift) as u8);

    if !low_only {
        let a_high = a.high.as_ref();
        let b_high = b.high.as_ref();

        // Main logic
        // sum_high = ids.a.high + ids.b.high + ids.carry_low
        // ids.carry_high = 1 if sum_high >= ids.SHIFT else 0
        let carry_high = Felt252::from((a_high + b_high + carry_low >= shift) as u8);

        insert_value_from_var_name("carry_high", carry_high, vm, ids_data, ap_tracking)?;
    }

    insert_value_from_var_name("carry_low", carry_low, vm, ids_data, ap_tracking)
}

/*
Implements hint:
%{
    res = ids.a + ids.b
    ids.carry = 1 if res >= ids.SHIFT else 0
%}
*/
pub fn uint128_add(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let shift = pow2_const(128);
    let a = get_integer_from_var_name("a", vm, ids_data, ap_tracking)?;
    let b = get_integer_from_var_name("b", vm, ids_data, ap_tracking)?;
    let a = a.as_ref();
    let b = b.as_ref();

    // Main logic
    // res = ids.a + ids.b
    // ids.carry = 1 if res >= ids.SHIFT else 0
    let carry = Felt252::from((a + b >= shift) as u8);

    insert_value_from_var_name("carry", carry, vm, ids_data, ap_tracking)
}

/*
Implements hint:
%{
    def split(num: int, num_bits_shift: int = 128, length: int = 2):
        a = []
        for _ in range(length):
            a.append( num & ((1 << num_bits_shift) - 1) )
            num = num >> num_bits_shift
        return tuple(a)

    def pack(z, num_bits_shift: int = 128) -> int:
        limbs = (z.low, z.high)
        return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

    a = pack(ids.a)
    b = pack(ids.b)
    res = (a - b)%2**256
    res_split = split(res)
    ids.res.low = res_split[0]
    ids.res.high = res_split[1]
%}
*/

pub fn uint256_sub(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let a = Uint256::from_var_name("a", vm, ids_data, ap_tracking)?.pack();
    let b = Uint256::from_var_name("b", vm, ids_data, ap_tracking)?.pack();

    // Main logic:
    // res = (a - b)%2**256
    let res = if a >= b {
        a - b
    } else {
        // wrapped a - b
        // b is limited to (CAIRO_PRIME - 1) << 128 which is 1 << (251 + 128 + 1)
        //                                         251: most significant felt bit
        //                                         128:     high field left shift
        //                                           1:       extra bit for limit
        let mod_256 = BigUint::one() << 256;
        if mod_256 >= b {
            mod_256 - b + a
        } else {
            let lowered_b = b.mod_floor(&mod_256);
            // Repeat the logic from before
            if a >= lowered_b {
                a - lowered_b
            } else {
                mod_256 - lowered_b + a
            }
        }
    };

    let res = Uint256::split(&res);

    res.insert_from_var_name("res", vm, ids_data, ap_tracking)
}

/*
Implements hint:
%{
    ids.low = ids.a & ((1<<64) - 1)
    ids.high = ids.a >> 64
%}
*/
pub fn split_64(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let a = get_integer_from_var_name("a", vm, ids_data, ap_tracking)?;
    let digits = a.to_le_digits();
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(digits[1].to_le_bytes().as_slice());
    bytes[8..16].copy_from_slice(digits[2].to_le_bytes().as_slice());
    bytes[16..24].copy_from_slice(digits[3].to_le_bytes().as_slice());

    let low = Felt252::from(digits[0]);
    let high = Felt252::from_bytes_le(&bytes);
    insert_value_from_var_name("high", high, vm, ids_data, ap_tracking)?;
    insert_value_from_var_name("low", low, vm, ids_data, ap_tracking)
}

/*
Implements hint:
%{
    from starkware.python.math_utils import isqrt
    n = (ids.n.high << 128) + ids.n.low
    root = isqrt(n)
    assert 0 <= root < 2 ** 128
    ids.root.low = root
    ids.root.high = 0
%}
*/
pub fn uint256_sqrt(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    only_low: bool,
) -> Result<(), HintError> {
    let n = Uint256::from_var_name("n", vm, ids_data, ap_tracking)?.pack();

    // Main logic
    // from starkware.python.math_utils import isqrt
    // n = (ids.n.high << 128) + ids.n.low
    // root = isqrt(n)
    // assert 0 <= root < 2 ** 128
    // ids.root.low = root
    // ids.root.high = 0

    let root = isqrt(&n)?;

    if root.bits() > 128 {
        return Err(HintError::AssertionFailed(
            format!("assert 0 <= {} < 2 ** 128", &root).into_boxed_str(),
        ));
    }

    let root = Felt252::from(&root);

    if only_low {
        insert_value_from_var_name("root", root, vm, ids_data, ap_tracking)?;
    } else {
        let root_u256 = Uint256::from_values(root, Felt252::ZERO);
        root_u256.insert_from_var_name("root", vm, ids_data, ap_tracking)?;
    }
    Ok(())
}

/*
Implements hint:
%{ memory[ap] = 1 if 0 <= (ids.a.high % PRIME) < 2 ** 127 else 0 %}
*/
pub fn uint256_signed_nn(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let a_addr = get_relocatable_from_var_name("a", vm, ids_data, ap_tracking)?;
    let a_high = vm.get_integer((a_addr + 1_usize)?)?;
    //Main logic
    //memory[ap] = 1 if 0 <= (ids.a.high % PRIME) < 2 ** 127 else 0
    let result: Felt252 =
        if *a_high >= Felt252::ZERO && a_high.as_ref() <= &Felt252::from(i128::MAX) {
            Felt252::ONE
        } else {
            Felt252::ZERO
        };
    insert_value_into_ap(vm, result)
}

/*
Implements hint:
%{
    a = (ids.a.high << 128) + ids.a.low
    div = (ids.div.high << 128) + ids.div.low
    quotient, remainder = divmod(a, div)

    ids.quotient.low = quotient & ((1 << 128) - 1)
    ids.quotient.high = quotient >> 128
    ids.remainder.low = remainder & ((1 << 128) - 1)
    ids.remainder.high = remainder >> 128
%}
*/
pub fn uint256_unsigned_div_rem(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    uint256_offseted_unsigned_div_rem(vm, ids_data, ap_tracking, 0, 1)
}

/*
Implements hint:
%{
    a = (ids.a.high << 128) + ids.a.low
    div = (ids.div.b23 << 128) + ids.div.b01
    quotient, remainder = divmod(a, div)

    ids.quotient.low = quotient & ((1 << 128) - 1)
    ids.quotient.high = quotient >> 128
    ids.remainder.low = remainder & ((1 << 128) - 1)
    ids.remainder.high = remainder >> 128
%}
*/
pub fn uint256_expanded_unsigned_div_rem(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    uint256_offseted_unsigned_div_rem(vm, ids_data, ap_tracking, 1, 3)
}

pub fn uint256_offseted_unsigned_div_rem(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    div_offset_low: usize,
    div_offset_high: usize,
) -> Result<(), HintError> {
    let a = Uint256::from_var_name("a", vm, ids_data, ap_tracking)?;
    let a_low = a.low.as_ref();
    let a_high = a.high.as_ref();

    let div_addr = get_relocatable_from_var_name("div", vm, ids_data, ap_tracking)?;
    let div_low = vm.get_integer((div_addr + div_offset_low)?)?;
    let div_high = vm.get_integer((div_addr + div_offset_high)?)?;
    let div_low = div_low.as_ref();
    let div_high = div_high.as_ref();

    //Main logic
    //a = (ids.a.high << 128) + ids.a.low
    //div = (ids.div.high << 128) + ids.div.low
    //quotient, remainder = divmod(a, div)

    //ids.quotient.low = quotient & ((1 << 128) - 1)
    //ids.quotient.high = quotient >> 128
    //ids.remainder.low = remainder & ((1 << 128) - 1)
    //ids.remainder.high = remainder >> 128

    let a = (a_high.to_biguint() << 128_u32) + a_low.to_biguint();
    let div = (div_high.to_biguint() << 128_u32) + div_low.to_biguint();
    //a and div will always be positive numbers
    //Then, Rust div_rem equals Python divmod
    let (quotient, remainder) = div_rem(a, div);

    let quotient = Uint256::from(&quotient);
    let remainder = Uint256::from(&remainder);

    quotient.insert_from_var_name("quotient", vm, ids_data, ap_tracking)?;
    remainder.insert_from_var_name("remainder", vm, ids_data, ap_tracking)?;

    Ok(())
}

/* Implements Hint:
%{
a = (ids.a.high << 128) + ids.a.low
b = (ids.b.high << 128) + ids.b.low
div = (ids.div.high << 128) + ids.div.low
quotient, remainder = divmod(a * b, div)

ids.quotient_low.low = quotient & ((1 << 128) - 1)
ids.quotient_low.high = (quotient >> 128) & ((1 << 128) - 1)
ids.quotient_high.low = (quotient >> 256) & ((1 << 128) - 1)
ids.quotient_high.high = quotient >> 384
ids.remainder.low = remainder & ((1 << 128) - 1)
ids.remainder.high = remainder >> 128
%}
*/
pub fn uint256_mul_div_mod(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    // Extract variables
    let a_addr = get_relocatable_from_var_name("a", vm, ids_data, ap_tracking)?;
    let b_addr = get_relocatable_from_var_name("b", vm, ids_data, ap_tracking)?;
    let div_addr = get_relocatable_from_var_name("div", vm, ids_data, ap_tracking)?;
    let quotient_low_addr =
        get_relocatable_from_var_name("quotient_low", vm, ids_data, ap_tracking)?;
    let quotient_high_addr =
        get_relocatable_from_var_name("quotient_high", vm, ids_data, ap_tracking)?;
    let remainder_addr = get_relocatable_from_var_name("remainder", vm, ids_data, ap_tracking)?;

    let a_low = vm.get_integer(a_addr)?;
    let a_high = vm.get_integer((a_addr + 1_usize)?)?;
    let b_low = vm.get_integer(b_addr)?;
    let b_high = vm.get_integer((b_addr + 1_usize)?)?;
    let div_low = vm.get_integer(div_addr)?;
    let div_high = vm.get_integer((div_addr + 1_usize)?)?;
    let a_low = a_low.as_ref();
    let a_high = a_high.as_ref();
    let b_low = b_low.as_ref();
    let b_high = b_high.as_ref();
    let div_low = div_low.as_ref();
    let div_high = div_high.as_ref();

    // Main Logic
    let a = a_high.to_biguint().shl(128_usize) + a_low.to_biguint();
    let b = b_high.to_biguint().shl(128_usize) + b_low.to_biguint();
    let div = div_high.to_biguint().shl(128_usize) + div_low.to_biguint();
    if div.is_zero() {
        return Err(MathError::DividedByZero.into());
    }
    let (quotient, remainder) = (a * b).div_mod_floor(&div);

    // ids.quotient_low.low
    vm.insert_value(
        quotient_low_addr,
        Felt252::from(&(&quotient & &BigUint::from(u128::MAX))),
    )?;
    // ids.quotient_low.high
    vm.insert_value(
        (quotient_low_addr + 1)?,
        Felt252::from(&((&quotient).shr(128_u32) & &BigUint::from(u128::MAX))),
    )?;
    // ids.quotient_high.low
    vm.insert_value(
        quotient_high_addr,
        Felt252::from(&((&quotient).shr(256_u32) & &BigUint::from(u128::MAX))),
    )?;
    // ids.quotient_high.high
    vm.insert_value(
        (quotient_high_addr + 1)?,
        Felt252::from(&((&quotient).shr(384_u32))),
    )?;
    //ids.remainder.low
    vm.insert_value(
        remainder_addr,
        Felt252::from(&(&remainder & &BigUint::from(u128::MAX))),
    )?;
    //ids.remainder.high
    vm.insert_value(
        (remainder_addr + 1)?,
        Felt252::from(&remainder.shr(128_u32)),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::felt_str;
    use crate::{
        any_box,
        hint_processor::{
            builtin_hint_processor::{
                builtin_hint_processor_definition::{BuiltinHintProcessor, HintProcessorData},
                hint_code,
            },
            hint_processor_definition::HintProcessorLogic,
        },
        types::relocatable::{MaybeRelocatable, Relocatable},
        utils::test_utils::*,
        vm::{errors::memory_errors::MemoryError, vm_core::VirtualMachine},
    };
    use assert_matches::assert_matches;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_uint256_add_ok() {
        let hint_code = hint_code::UINT256_ADD;
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create hint_data
        let ids_data =
            non_continuous_ids_data![("a", -6), ("b", -4), ("carry_low", 2), ("carry_high", 3)];
        vm.segments = segments![
            ((1, 4), 2),
            ((1, 5), 3),
            ((1, 6), 4),
            ((1, 7), ("340282366920938463463374607431768211455", 10))
        ];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory inserts
        check_memory![vm.segments.memory, ((1, 12), 0), ((1, 13), 1)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_uint256_add_low_only_ok() {
        let hint_code =
            "sum_low = ids.a.low + ids.b.low\nids.carry_low = 1 if sum_low >= ids.SHIFT else 0";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("a", -6), ("b", -4), ("carry_low", 2)];
        vm.segments = segments![
            ((1, 4), 2),
            ((1, 5), 3),
            ((1, 6), 4),
            ((1, 7), ("340282366920938463463374607431768211455", 10))
        ];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory inserts
        check_memory![vm.segments.memory, ((1, 12), 0)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_uint128_add_ok() {
        let hint_code = hint_code::UINT128_ADD;
        let mut vm = vm_with_range_check!();
        // Initialize fp
        vm.run_context.fp = 0;
        // Create hint_data
        let ids_data = non_continuous_ids_data![("a", 0), ("b", 1), ("carry", 2)];
        vm.segments = segments![
            ((1, 0), 180141183460469231731687303715884105727_u128),
            ((1, 1), 180141183460469231731687303715884105727_u128),
        ];
        // Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        // Check hint memory inserts
        check_memory![vm.segments.memory, ((1, 2), 1)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_uint256_add_fail_inserts() {
        let hint_code = hint_code::UINT256_ADD;
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create hint_data
        let ids_data =
            non_continuous_ids_data![("a", -6), ("b", -4), ("carry_high", 3), ("carry_low", 2)];
        //Insert ids into memory
        vm.segments = segments![
            ((1, 4), 2),
            ((1, 5), 3),
            ((1, 6), 4),
            ((1, 7), 2),
            ((1, 12), 2)
        ];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(
                MemoryError::InconsistentMemory(bx)
            )) if *bx == (Relocatable::from((1, 12)),
                    MaybeRelocatable::from(Felt252::from(2)),
                    MaybeRelocatable::from(Felt252::ZERO))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_uint256_sub_nonnegative_ok() {
        let hint_code = hint_code::UINT256_SUB;
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 0;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("a", 0), ("b", 2), ("res", 4)];
        vm.segments = segments![
            ((1, 0), 12179),
            ((1, 1), 13044),
            ((1, 2), 1001),
            ((1, 3), 6687),
        ];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory inserts
        check_memory![vm.segments.memory, ((1, 4), 11178), ((1, 5), 6357)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_uint256_sub_negative_ok() {
        let hint_code = hint_code::UINT256_SUB;
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 0;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("a", 0), ("b", 2), ("res", 4)];
        vm.segments = segments![
            ((1, 0), 1001),
            ((1, 1), 6687),
            ((1, 2), 12179),
            ((1, 3), 13044),
        ];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory inserts
        check_memory![
            vm.segments.memory,
            ((1, 4), ("340282366920938463463374607431768200278", 10)),
            ((1, 5), ("340282366920938463463374607431768205098", 10))
        ];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_uint256_sub_b_high_gt_256_lte_a() {
        let hint_code = hint_code::UINT256_SUB;
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 0;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("a", 0), ("b", 2), ("res", 4)];
        vm.segments = segments![
            ((1, 0), ("340282366920938463463374607431768211456", 10)),
            ((1, 1), 0),
            ((1, 2), 0),
            ((1, 3), ("340282366920938463463374607431768211457", 10)),
        ];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory inserts
        check_memory![vm.segments.memory, ((1, 4), 0), ((1, 5), 0)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_uint256_sub_b_high_gt_256_gt_a() {
        let hint_code = hint_code::UINT256_SUB;
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 0;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("a", 0), ("b", 2), ("res", 4)];
        vm.segments = segments![
            ((1, 0), 1),
            ((1, 1), 0),
            ((1, 2), 0),
            ((1, 3), ("340282366920938463463374607431768211457", 10)),
        ];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory inserts
        check_memory![
            vm.segments.memory,
            ((1, 4), ("1", 10)),
            ((1, 5), ("340282366920938463463374607431768211455", 10))
        ];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_uint256_sub_missing_member() {
        let hint_code = hint_code::UINT256_SUB;
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 0;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("a", 0)];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data.clone(), hint_code),
            Err(HintError::IdentifierHasNoMember(bx)) if *bx == ("a".to_string(), "low".to_string())
        );
        vm.segments = segments![((1, 0), 1001)];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code),
            Err(HintError::IdentifierHasNoMember(bx)) if *bx == ("a".to_string(), "high".to_string())
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_split_64_ok() {
        let hint_code = "ids.low = ids.a & ((1<<64) - 1)\nids.high = ids.a >> 64";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("a", -3), ("high", 1), ("low", 0)];
        //Insert ids.a into memory
        vm.segments = segments![((1, 7), ("850981239023189021389081239089023", 10))];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory inserts
        //ids.low, ids.high
        check_memory![
            vm.segments.memory,
            ((1, 10), 7249717543555297151_u64),
            ((1, 11), 46131785404667_u64)
        ];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_split_64_with_big_a() {
        let hint_code = "ids.low = ids.a & ((1<<64) - 1)\nids.high = ids.a >> 64";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create ids_data
        let ids_data = non_continuous_ids_data![("a", -3), ("high", 1), ("low", 0)];
        //Insert ids.a into memory
        vm.segments = segments![((1, 7), ("400066369019890261321163226850167045262", 10))];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));

        //Check hint memory inserts
        //ids.low, ids.high
        check_memory![
            vm.segments.memory,
            ((1, 10), 2279400676465785998_u64),
            ((1, 11), 21687641321487626429_u128)
        ];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_split_64_memory_error() {
        let hint_code = "ids.low = ids.a & ((1<<64) - 1)\nids.high = ids.a >> 64";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("a", -3), ("high", 1), ("low", 0)];
        //Insert ids.a into memory
        vm.segments = segments![
            ((1, 7), ("850981239023189021389081239089023", 10)),
            ((1, 10), 0)
        ];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(
                MemoryError::InconsistentMemory(bx)
            )) if *bx == (Relocatable::from((1, 10)),
                    MaybeRelocatable::from(Felt252::ZERO),
                    MaybeRelocatable::from(felt_str!("7249717543555297151")))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_uint256_sqrt_ok() {
        let hint_code = hint_code::UINT256_SQRT;
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 5;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("n", -5), ("root", 0)];
        vm.segments = segments![((1, 0), 17), ((1, 1), 7)];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory inserts
        //ids.root.low, ids.root.high
        check_memory![
            vm.segments.memory,
            ((1, 5), 48805497317890012913_u128),
            ((1, 6), 0)
        ];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_uint256_sqrt_felt_ok() {
        let hint_code = "from starkware.python.math_utils import isqrt\nn = (ids.n.high << 128) + ids.n.low\nroot = isqrt(n)\nassert 0 <= root < 2 ** 128\nids.root = root";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 0;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("n", 0), ("root", 2)];
        vm.segments = segments![((1, 0), 879232), ((1, 1), 135906)];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory inserts
        //ids.root
        check_memory![vm.segments.memory, ((1, 2), 6800471701195223914689)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_uint256_sqrt_assert_error() {
        let hint_code = "from starkware.python.math_utils import isqrt\nn = (ids.n.high << 128) + ids.n.low\nroot = isqrt(n)\nassert 0 <= root < 2 ** 128\nids.root.low = root\nids.root.high = 0";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 5;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("n", -5), ("root", 0)];
        vm.segments = segments![
            ((1, 0), 0),
            ((1, 1), ("340282366920938463463374607431768211458", 10))
        ];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::AssertionFailed(bx)) if bx.as_ref() == "assert 0 <= 340282366920938463463374607431768211456 < 2 ** 128"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_uint256_invalid_memory_insert() {
        let hint_code = "from starkware.python.math_utils import isqrt\nn = (ids.n.high << 128) + ids.n.low\nroot = isqrt(n)\nassert 0 <= root < 2 ** 128\nids.root.low = root\nids.root.high = 0";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 5;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("n", -5), ("root", 0)];
        //Insert  ids.n.low into memory
        vm.segments = segments![((1, 0), 17), ((1, 1), 7), ((1, 5), 1)];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(
                MemoryError::InconsistentMemory(bx)
            )) if *bx == (Relocatable::from((1, 5)),
                    MaybeRelocatable::from(Felt252::ONE),
                    MaybeRelocatable::from(felt_str!("48805497317890012913")))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_signed_nn_ok_result_one() {
        let hint_code = "memory[ap] = 1 if 0 <= (ids.a.high % PRIME) < 2 ** 127 else 0";
        let mut vm = vm_with_range_check!();
        //Initialize run_context
        run_context!(vm, 0, 5, 4);
        //Create hint_data
        let ids_data = non_continuous_ids_data![("a", -4)];
        //Insert ids.a.high into memory
        vm.segments = segments![(
            (1, 1),
            (
                "3618502788666131213697322783095070105793248398792065931704779359851756126208",
                10
            )
        )];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory insert
        //memory[ap] = 1 if 0 <= (ids.a.high % PRIME) < 2 ** 127 else 0
        check_memory![vm.segments.memory, ((1, 5), 1)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_signed_nn_ok_result_zero() {
        let hint_code = "memory[ap] = 1 if 0 <= (ids.a.high % PRIME) < 2 ** 127 else 0";
        let mut vm = vm_with_range_check!();
        //Initialize run_context
        run_context!(vm, 0, 5, 4);
        //Create hint_data
        let ids_data = non_continuous_ids_data![("a", -4)];
        //Insert ids.a.high into memory
        vm.segments = segments![(
            (1, 1),
            (
                "3618502788666131213697322783095070105793248398792065931704779359851756126209",
                10
            )
        )];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory insert
        //memory[ap] = 1 if 0 <= (ids.a.high % PRIME) < 2 ** 127 else 0
        check_memory![vm.segments.memory, ((1, 5), 0)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_signed_nn_ok_invalid_memory_insert() {
        let hint_code = "memory[ap] = 1 if 0 <= (ids.a.high % PRIME) < 2 ** 127 else 0";
        let mut vm = vm_with_range_check!();
        //Initialize run_context
        run_context!(vm, 0, 5, 4);
        //Create hint_data
        let ids_data = non_continuous_ids_data![("a", -4)];
        vm.segments = segments![((1, 1), 1), ((1, 5), 55)];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(
                MemoryError::InconsistentMemory(bx)
            )) if *bx == (Relocatable::from((1, 5)),
                    MaybeRelocatable::from(Felt252::from(55)),
                    MaybeRelocatable::from(Felt252::ONE))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_unsigned_div_rem_ok() {
        let hint_code = hint_code::UINT256_UNSIGNED_DIV_REM;
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create hint_data
        let ids_data =
            non_continuous_ids_data![("a", -6), ("div", -4), ("quotient", 0), ("remainder", 2)];
        //Insert ids into memory
        vm.segments = segments![((1, 4), 89), ((1, 5), 72), ((1, 6), 3), ((1, 7), 7)];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory inserts
        //ids.quotient.low, ids.quotient.high, ids.remainder.low, ids.remainder.high
        check_memory![
            vm.segments.memory,
            ((1, 10), 10),
            ((1, 11), 0),
            ((1, 12), 59),
            ((1, 13), 2)
        ];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_unsigned_div_rem_expanded_ok() {
        let hint_code = hint_code::UINT256_EXPANDED_UNSIGNED_DIV_REM;
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 0;
        //Create hint_data
        let ids_data =
            non_continuous_ids_data![("a", 0), ("div", 2), ("quotient", 7), ("remainder", 9)];
        //Insert ids into memory
        vm.segments = segments![
            // (72 << 128) + 89
            ((1, 0), 89),
            ((1, 1), 72),
            // uint256_expand((7 << 128) + 3)
            ((1, 2), 55340232221128654848),
            ((1, 3), 3),
            ((1, 4), 129127208515966861312),
            ((1, 5), 7),
            ((1, 6), 0),
        ];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory inserts
        //ids.quotient.low, ids.quotient.high, ids.remainder.low, ids.remainder.high
        check_memory![
            vm.segments.memory,
            ((1, 7), 10),
            ((1, 8), 0),
            ((1, 9), 59),
            ((1, 10), 2),
        ];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_unsigned_div_rem_invalid_memory_insert() {
        let hint_code = hint_code::UINT256_UNSIGNED_DIV_REM;
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create hint_data
        let ids_data =
            non_continuous_ids_data![("a", -6), ("div", -4), ("quotient", 0), ("remainder", 2)];
        //Insert ids into memory
        vm.segments = segments![
            ((1, 4), 89),
            ((1, 5), 72),
            ((1, 6), 3),
            ((1, 7), 7),
            ((1, 10), 0)
        ];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(
                MemoryError::InconsistentMemory(bx)
            )) if *bx == (Relocatable::from((1, 10)),
                    MaybeRelocatable::from(Felt252::ZERO),
                    MaybeRelocatable::from(Felt252::from(10)))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_unsigned_div_rem_invalid_memory_insert_2() {
        let hint_code = hint_code::UINT256_UNSIGNED_DIV_REM;
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create hint_data
        let ids_data =
            non_continuous_ids_data![("a", -6), ("div", -4), ("quotient", 0), ("remainder", 2)];
        //Insert ids into memory
        vm.segments = segments![
            ((1, 4), 89),
            ((1, 5), 72),
            ((1, 6), 3),
            ((1, 7), 7),
            ((1, 11), 1)
        ];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(
                MemoryError::InconsistentMemory(bx)
            )) if *bx == (Relocatable::from((1, 11)),
                    MaybeRelocatable::from(Felt252::ONE),
                    MaybeRelocatable::from(Felt252::ZERO))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_mul_div_mod_ok() {
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create hint_data
        let ids_data = non_continuous_ids_data![
            ("a", -8),
            ("b", -6),
            ("div", -4),
            ("quotient_low", 0),
            ("quotient_high", 2),
            ("remainder", 4)
        ];
        //Insert ids into memory
        vm.segments = segments![
            ((1, 2), 89),
            ((1, 3), 72),
            ((1, 4), 3),
            ((1, 5), 7),
            ((1, 6), 107),
            ((1, 7), 114)
        ];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::UINT256_MUL_DIV_MOD),
            Ok(())
        );
        //Check hint memory inserts
        //ids.quotient.low, ids.quotient.high, ids.remainder.low, ids.remainder.high
        check_memory![
            vm.segments.memory,
            ((1, 10), 143276786071974089879315624181797141668),
            ((1, 11), 4),
            ((1, 12), 0),
            ((1, 13), 0),
            //((1, 14), 322372768661941702228460154409043568767),
            ((1, 15), 101)
        ];
        assert_eq!(
            vm.segments
                .memory
                .get_integer((1, 14).into())
                .unwrap()
                .as_ref(),
            &felt_str!("322372768661941702228460154409043568767")
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_mul_div_mod_missing_ids() {
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create hint_data
        let ids_data = non_continuous_ids_data![
            ("a", -8),
            ("b", -6),
            ("div", -4),
            ("quotient", 0),
            ("remainder", 2)
        ];
        //Insert ids into memory
        vm.segments = segments![
            ((1, 2), 89),
            ((1, 3), 72),
            ((1, 4), 3),
            ((1, 5), 7),
            ((1, 6), 107),
            ((1, 7), 114)
        ];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::UINT256_MUL_DIV_MOD),
            Err(HintError::UnknownIdentifier(bx)) if bx.as_ref() == "quotient_low"
        );
    }
}
