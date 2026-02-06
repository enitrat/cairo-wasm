use core::str::FromStr;

use super::{hint_utils::get_relocatable_from_var_name, secp::bigint_utils::BigInt3};
use crate::{
    hint_processor::hint_processor_definition::HintReference,
    serde::deserialize_program::ApTracking,
    types::relocatable::MaybeRelocatable,
    utils::CAIRO_PRIME,
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
    Felt252,
};
use crate::{
    stdlib::{collections::HashMap, prelude::*},
    types::exec_scope::ExecutionScopes,
};
use lazy_static::lazy_static;
use num_bigint::{BigInt, BigUint};
use num_integer::Integer;
use num_traits::FromPrimitive;
use num_traits::Signed;

lazy_static! {
    static ref BLS_BASE: BigInt = BigInt::from_u64(2).unwrap().pow(86);
    static ref BLS_PRIME: BigInt = BigInt::from_str(
        "52435875175126190479447740508185965837690552500527637822603658699938581184513"
    )
    .unwrap();
}
pub const WRITE_DIVMOD_SEGMENT: &str = r#"from starkware.starknet.core.os.data_availability.bls_utils import BLS_PRIME, pack, split

a = pack(ids.a, PRIME)
b = pack(ids.b, PRIME)

q, r = divmod(a * b, BLS_PRIME)

# By the assumption: |a|, |b| < 2**104 * ((2**86) ** 2 + 2**86 + 1) < 2**276.001.
# Therefore |q| <= |ab| / BLS_PRIME < 2**299.
# Hence the absolute value of the high limb of split(q) < 2**127.
segments.write_arg(ids.q.address_, split(q))
segments.write_arg(ids.res.address_, split(r))"#;

pub fn write_div_mod_segment(
    vm: &mut VirtualMachine,
    _exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    _constants: &HashMap<String, Felt252>,
) -> Result<(), HintError> {
    let a = bls_pack(
        &BigInt3::from_var_name("a", vm, ids_data, ap_tracking)?,
        &CAIRO_PRIME,
    );
    let b = bls_pack(
        &BigInt3::from_var_name("b", vm, ids_data, ap_tracking)?,
        &CAIRO_PRIME,
    );
    let (q, r) = (a * b).div_mod_floor(&BLS_PRIME);
    let q_reloc = get_relocatable_from_var_name("q", vm, ids_data, ap_tracking)?;
    let res_reloc = get_relocatable_from_var_name("res", vm, ids_data, ap_tracking)?;

    let q_arg: Vec<MaybeRelocatable> = bls_split(q)?
        .into_iter()
        .map(|ref n| Felt252::from(n).into())
        .collect::<Vec<MaybeRelocatable>>();
    let res_arg: Vec<MaybeRelocatable> = bls_split(r)?
        .into_iter()
        .map(|ref n| Felt252::from(n).into())
        .collect::<Vec<MaybeRelocatable>>();
    vm.write_arg(q_reloc, &q_arg).map_err(HintError::Memory)?;
    vm.write_arg(res_reloc, &res_arg)
        .map_err(HintError::Memory)?;
    Ok(())
}

fn bls_split(mut num: BigInt) -> Result<Vec<BigInt>, HintError> {
    let mut canonical = Vec::new();
    for _ in 0..2 {
        let (new_num, residue) = num.div_rem(&BLS_BASE);
        num = new_num;
        canonical.push(residue);
    }

    if num.abs() >= BigInt::from(1u128 << 127) {
        return Err(HintError::BlsSplitError(Box::new(num)));
    }

    canonical.push(num);
    Ok(canonical)
}

fn as_int(value: BigInt, prime: &BigUint) -> BigInt {
    let half_prime: BigInt = (prime / 2u32).into();
    let prime: BigInt = prime.clone().into();
    if value < half_prime {
        value
    } else {
        value - prime
    }
}

fn bls_pack(z: &BigInt3, prime: &BigUint) -> BigInt {
    z.limbs
        .iter()
        .enumerate()
        .map(|(i, limb)| as_int(limb.to_bigint(), prime) * &BLS_BASE.pow(i as u32))
        .sum()
}
