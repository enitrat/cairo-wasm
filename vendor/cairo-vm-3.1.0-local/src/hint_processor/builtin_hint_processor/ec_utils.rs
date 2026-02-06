use crate::stdlib::{borrow::Cow, boxed::Box, collections::HashMap, prelude::*};
use crate::utils::CAIRO_PRIME;
use crate::Felt252;
use crate::{
    hint_processor::{
        builtin_hint_processor::hint_utils::{
            get_integer_from_var_name, get_relocatable_from_var_name,
        },
        hint_processor_definition::HintReference,
    },
    serde::deserialize_program::ApTracking,
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};
use lazy_static::lazy_static;
use num_bigint::BigUint;
use num_bigint::ToBigInt;
use num_traits::{Num, One, Pow, ToPrimitive, Zero};
use sha2::{Digest, Sha256};

use super::hint_utils::get_ptr_from_var_name;

#[derive(Debug, PartialEq)]
struct EcPoint<'a> {
    x: Cow<'a, Felt252>,
    y: Cow<'a, Felt252>,
}
impl EcPoint<'_> {
    fn from_var_name<'a>(
        name: &'a str,
        vm: &'a VirtualMachine,
        ids_data: &'a HashMap<String, HintReference>,
        ap_tracking: &'a ApTracking,
    ) -> Result<EcPoint<'a>, HintError> {
        // Get first addr of EcPoint struct
        let point_addr = get_relocatable_from_var_name(name, vm, ids_data, ap_tracking)?;
        Ok(EcPoint {
            x: vm.get_integer(point_addr).map_err(|_| {
                HintError::IdentifierHasNoMember(Box::new((name.to_string(), "x".to_string())))
            })?,
            y: vm.get_integer((point_addr + 1)?).map_err(|_| {
                HintError::IdentifierHasNoMember(Box::new((name.to_string(), "y".to_string())))
            })?,
        })
    }
}

// Implements hint:
// from starkware.crypto.signature.signature import ALPHA, BETA, FIELD_PRIME
// from starkware.python.math_utils import random_ec_point
// from starkware.python.utils import to_bytes

// # Define a seed for random_ec_point that's dependent on all the input, so that:
// #   (1) The added point s is deterministic.
// #   (2) It's hard to choose inputs for which the builtin will fail.
// seed = b"".join(map(to_bytes, [ids.p.x, ids.p.y, ids.m, ids.q.x, ids.q.y]))
// ids.s.x, ids.s.y = random_ec_point(FIELD_PRIME, ALPHA, BETA, seed)

pub fn random_ec_point_hint(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let p = EcPoint::from_var_name("p", vm, ids_data, ap_tracking)?;
    let q = EcPoint::from_var_name("q", vm, ids_data, ap_tracking)?;
    let m = Cow::Owned(get_integer_from_var_name("m", vm, ids_data, ap_tracking)?);
    let bytes: Vec<u8> = [p.x, p.y, m, q.x, q.y]
        .iter()
        .flat_map(|x| x.to_bytes_be())
        .collect();
    let (x, y) = random_ec_point_seeded(bytes)?;
    let s_addr = get_relocatable_from_var_name("s", vm, ids_data, ap_tracking)?;
    vm.insert_value(s_addr, x)?;
    vm.insert_value((s_addr + 1)?, y)?;
    Ok(())
}

// Implements hint:
// from starkware.crypto.signature.signature import ALPHA, BETA, FIELD_PRIME
//     from starkware.python.math_utils import random_ec_point
//     from starkware.python.utils import to_bytes

//     n_elms = ids.len
//     assert isinstance(n_elms, int) and n_elms >= 0, \
//         f'Invalid value for len. Got: {n_elms}.'
//     if '__chained_ec_op_max_len' in globals():
//         assert n_elms <= __chained_ec_op_max_len, \
//             f'chained_ec_op() can only be used with len<={__chained_ec_op_max_len}. ' \
//             f'Got: n_elms={n_elms}.'

//     # Define a seed for random_ec_point that's dependent on all the input, so that:
//     #   (1) The added point s is deterministic.
//     #   (2) It's hard to choose inputs for which the builtin will fail.
//     seed = b"".join(
//         map(
//             to_bytes,
//             [
//                 ids.p.x,
//                 ids.p.y,
//                 *memory.get_range(ids.m, n_elms),
//                 *memory.get_range(ids.q.address_, 2 * n_elms),
//             ],
//         )
//     )
//     ids.s.x, ids.s.y = random_ec_point(FIELD_PRIME, ALPHA, BETA, seed)"
pub fn chained_ec_op_random_ec_point_hint(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let n_elms = get_integer_from_var_name("len", vm, ids_data, ap_tracking)?;
    if n_elms.is_zero() || n_elms.to_usize().is_none() {
        return Err(HintError::InvalidLenValue(Box::new(n_elms)));
    }
    let n_elms = n_elms.to_usize().unwrap();
    let p = EcPoint::from_var_name("p", vm, ids_data, ap_tracking)?;
    let m = get_ptr_from_var_name("m", vm, ids_data, ap_tracking)?;
    let q = get_ptr_from_var_name("q", vm, ids_data, ap_tracking)?;
    let m_range = vm.get_integer_range(m, n_elms)?;
    let q_range = vm.get_integer_range(q, n_elms * 2)?;
    let bytes: Vec<u8> = [p.x, p.y]
        .iter()
        .chain(m_range.iter())
        .chain(q_range.iter())
        .flat_map(|x| x.to_bytes_be())
        .collect();
    let (x, y) = random_ec_point_seeded(bytes)?;
    let s_addr = get_relocatable_from_var_name("s", vm, ids_data, ap_tracking)?;
    vm.insert_value(s_addr, x)?;
    vm.insert_value((s_addr + 1)?, y)?;
    Ok(())
}

// Implements hint:
// from starkware.crypto.signature.signature import ALPHA, BETA, FIELD_PRIME
// from starkware.python.math_utils import recover_y
// ids.p.x = ids.x
// # This raises an exception if `x` is not on the curve.
// ids.p.y = recover_y(ids.x, ALPHA, BETA, FIELD_PRIME)
pub fn recover_y_hint(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let p_x = get_integer_from_var_name("x", vm, ids_data, ap_tracking)?;
    let p_addr = get_relocatable_from_var_name("p", vm, ids_data, ap_tracking)?;
    vm.insert_value(p_addr, p_x)?;
    let p_y = Felt252::from(
        &recover_y(&p_x.to_biguint())
            .ok_or_else(|| HintError::RecoverYPointNotOnCurve(Box::new(p_x)))?,
    );
    vm.insert_value((p_addr + 1)?, p_y)?;
    Ok(())
}

// Returns a random non-zero point on the elliptic curve
//   y^2 = x^3 + alpha * x + beta (mod field_prime).
// The point is created deterministically from the seed.
fn random_ec_point_seeded(seed_bytes: Vec<u8>) -> Result<(Felt252, Felt252), HintError> {
    // Hash initial seed
    let mut hasher = Sha256::new();
    hasher.update(seed_bytes);
    let seed = hasher.finalize_reset().to_vec();
    for i in 0..100 {
        // Calculate x
        let i_bytes = (i as u8).to_le_bytes();
        let mut input = seed[1..].to_vec();
        input.extend(i_bytes);
        input.extend(vec![0; 10 - i_bytes.len()]);
        hasher.update(input);
        let x = BigUint::from_bytes_be(&hasher.finalize_reset());
        // Calculate y
        let y_coef = (-1).pow(seed[0] & 1);
        let y = recover_y(&x);
        if let Some(y) = y {
            // Conversion from BigUint to BigInt doesnt fail
            return Ok((
                Felt252::from(&x),
                Felt252::from(&(y.to_bigint().unwrap() * y_coef)),
            ));
        }
    }
    Err(HintError::RandomEcPointNotOnCurve)
}
const ALPHA: u32 = 1;
lazy_static! {
    static ref BETA: BigUint = BigUint::from_str_radix(
        "3141592653589793238462643383279502884197169399375105820974944592307816406665",
        10
    )
    .unwrap();
    static ref FELT_MAX_HALVED: BigUint = Felt252::MAX.to_biguint() / 2_u32;
}

// Recovers the corresponding y coordinate on the elliptic curve
//     y^2 = x^3 + alpha * x + beta (mod field_prime)
//     of a given x coordinate.
// Returns None if x is not the x coordinate of a point in the curve
fn recover_y(x: &BigUint) -> Option<BigUint> {
    let y_squared: BigUint = x.modpow(&BigUint::from(3_u32), &CAIRO_PRIME) + ALPHA * x + &*BETA;
    if is_quad_residue(&y_squared) {
        Some(Felt252::from(&y_squared).sqrt()?.to_biguint())
    } else {
        None
    }
}

// Implementation adapted from sympy implementation
// Conditions:
// + prime is ommited as it will be CAIRO_PRIME
// + a >= 0 < prime (other cases ommited)
fn is_quad_residue(a: &BigUint) -> bool {
    a.is_zero() || a.is_one() || a.modpow(&FELT_MAX_HALVED, &CAIRO_PRIME).is_one()
}

#[cfg(test)]
mod tests {
    use crate::any_box;
    use crate::felt_str;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::HintProcessorData;
    use crate::hint_processor::hint_processor_definition::HintProcessorLogic;
    use crate::relocatable;
    use crate::types::relocatable::Relocatable;
    use num_traits::Zero;

    use crate::hint_processor::builtin_hint_processor::hint_code;
    use crate::utils::test_utils::*;
    use assert_matches::assert_matches;

    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    fn test_is_quad_residue_less_than_2() {
        assert!(is_quad_residue(&BigUint::one()));
        assert!(is_quad_residue(&BigUint::zero()));
    }

    #[test]
    fn test_is_quad_residue_false() {
        assert!(!is_quad_residue(
            &BigUint::from_str_radix(
                "205857351767627712295703269674687767888261140702556021834663354704341414042",
                10
            )
            .unwrap()
        ));
    }

    #[test]
    fn test_is_quad_residue_true() {
        assert!(is_quad_residue(
            &BigUint::from_str_radix(
                "99957092485221722822822221624080199277265330641980989815386842231144616633668",
                10
            )
            .unwrap()
        ));
    }

    #[test]
    fn test_recover_y_valid() {
        let x = BigUint::from_str_radix(
            "2497468900767850684421727063357792717599762502387246235265616708902555305129",
            10,
        )
        .unwrap();
        let y = BigUint::from_str_radix(
            "205857351767627712295703269674687767888261140702556021834663354704341414042",
            10,
        )
        .unwrap();
        assert_eq!(recover_y(&x), Some(y));
    }

    #[test]
    fn test_recover_y_invalid() {
        let x = BigUint::from_str_radix(
            "205857351767627712295703269674687767888261140702556021834663354704341414042",
            10,
        )
        .unwrap();
        assert_eq!(recover_y(&x), None);
    }

    #[test]
    fn get_random_ec_point_seeded() {
        let seed: Vec<u8> = vec![
            6, 164, 190, 174, 245, 169, 52, 37, 185, 115, 23, 156, 219, 160, 201, 212, 47, 48, 224,
            26, 95, 30, 45, 183, 61, 160, 136, 75, 141, 103, 86, 252, 7, 37, 101, 236, 129, 188, 9,
            255, 83, 251, 250, 217, 147, 36, 169, 42, 165, 179, 159, 181, 130, 103, 227, 149, 232,
            171, 227, 98, 144, 235, 242, 79, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 34, 6, 84, 253, 126, 103, 161, 35, 221, 19, 134,
            128, 147, 179, 183, 119, 127, 31, 254, 245, 150, 194, 227, 36, 242, 92, 234, 249, 20,
            102, 152, 72, 44, 4, 250, 210, 105, 203, 248, 96, 152, 14, 56, 118, 143, 233, 203, 107,
            11, 154, 176, 62, 227, 254, 132, 207, 222, 46, 204, 206, 89, 124, 135, 79, 216,
        ];
        let x = felt_str!(
            "2497468900767850684421727063357792717599762502387246235265616708902555305129"
        );
        let y = felt_str!(
            "3412645436898503501401619513420382337734846074629040678138428701431530606439"
        );
        assert_eq!(random_ec_point_seeded(seed).unwrap(), (x, y));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_ec_op_random_ec_point_hint() {
        let hint_code = hint_code::RANDOM_EC_POINT;
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 6;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("p", -6), ("q", -3), ("m", -4), ("s", -1)];
        /*  p.x = 3004956058830981475544150447242655232275382685012344776588097793621230049020
           p.y = 3232266734070744637901977159303149980795588196503166389060831401046564401743
           m = 34
           q.x = 2864041794633455918387139831609347757720597354645583729611044800117714995244
           q.y = 2252415379535459416893084165764951913426528160630388985542241241048300343256
        */
        add_segments!(vm, 2);
        vm.insert_value(
            (1, 0).into(),
            felt_str!(
                "3004956058830981475544150447242655232275382685012344776588097793621230049020"
            ),
        )
        .unwrap();
        vm.insert_value(
            (1, 1).into(),
            felt_str!(
                "3232266734070744637901977159303149980795588196503166389060831401046564401743"
            ),
        )
        .unwrap();
        vm.insert_value((1, 2).into(), Felt252::from(34)).unwrap();
        vm.insert_value(
            (1, 3).into(),
            felt_str!(
                "2864041794633455918387139831609347757720597354645583729611044800117714995244"
            ),
        )
        .unwrap();
        vm.insert_value(
            (1, 4).into(),
            felt_str!(
                "2252415379535459416893084165764951913426528160630388985542241241048300343256"
            ),
        )
        .unwrap();
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        // Check post-hint memory values
        // s.x = 96578541406087262240552119423829615463800550101008760434566010168435227837635
        // s.y = 3412645436898503501401619513420382337734846074629040678138428701431530606439
        assert_eq!(
            vm.get_integer((1, 5).into()).unwrap().as_ref(),
            &felt_str!(
                "96578541406087262240552119423829615463800550101008760434566010168435227837635"
            )
        );
        assert_eq!(
            vm.get_integer((1, 6).into()).unwrap().as_ref(),
            &felt_str!(
                "3412645436898503501401619513420382337734846074629040678138428701431530606439"
            )
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_chained_ec_op_random_ec_point_hint() {
        let hint_code = hint_code::CHAINED_EC_OP_RANDOM_EC_POINT;
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 6;
        //Create hint_data
        let ids_data =
            non_continuous_ids_data![("p", -6), ("m", -4), ("q", -3), ("len", -2), ("s", -1)];
        /*
            p.x = 3004956058830981475544150447242655232275382685012344776588097793621230049020
            p.y = 3232266734070744637901977159303149980795588196503166389060831401046564401743
            _m = 34
            -q.x = 2864041794633455918387139831609347757720597354645583729611044800117714995244
            -q.y = 2252415379535459416893084165764951913426528160630388985542241241048300343256
            q = [q,q,q]
            m = [m,m,m]
            len = 3
        */
        add_segments!(vm, 4);
        //p
        vm.insert_value(
            (1, 0).into(),
            felt_str!(
                "3004956058830981475544150447242655232275382685012344776588097793621230049020"
            ),
        )
        .unwrap();
        vm.insert_value(
            (1, 1).into(),
            felt_str!(
                "3232266734070744637901977159303149980795588196503166389060831401046564401743"
            ),
        )
        .unwrap();
        //m
        vm.insert_value((1, 2).into(), relocatable!(2, 0)).unwrap();
        vm.insert_value((2, 0).into(), Felt252::from(34)).unwrap();
        vm.insert_value((2, 1).into(), Felt252::from(34)).unwrap();
        vm.insert_value((2, 2).into(), Felt252::from(34)).unwrap();
        //q
        vm.insert_value((1, 3).into(), relocatable!(3, 0)).unwrap();
        vm.insert_value(
            (3, 0).into(),
            felt_str!(
                "2864041794633455918387139831609347757720597354645583729611044800117714995244"
            ),
        )
        .unwrap();
        vm.insert_value(
            (3, 1).into(),
            felt_str!(
                "2252415379535459416893084165764951913426528160630388985542241241048300343256"
            ),
        )
        .unwrap();
        vm.insert_value(
            (3, 2).into(),
            felt_str!(
                "2864041794633455918387139831609347757720597354645583729611044800117714995244"
            ),
        )
        .unwrap();
        vm.insert_value(
            (3, 3).into(),
            felt_str!(
                "2252415379535459416893084165764951913426528160630388985542241241048300343256"
            ),
        )
        .unwrap();
        vm.insert_value(
            (3, 4).into(),
            felt_str!(
                "2864041794633455918387139831609347757720597354645583729611044800117714995244"
            ),
        )
        .unwrap();
        vm.insert_value(
            (3, 5).into(),
            felt_str!(
                "2252415379535459416893084165764951913426528160630388985542241241048300343256"
            ),
        )
        .unwrap();
        //len
        vm.insert_value((1, 4).into(), Felt252::from(3)).unwrap();
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        // Check post-hint memory values
        // s.x = 1354562415074475070179359167082942891834423311678180448592849484844152837347
        // s.y = 907662328694455187848008017177970257426839229889571025406355869359245158736
        assert_eq!(
            vm.get_integer((1, 5).into()).unwrap().as_ref(),
            &felt_str!(
                "1354562415074475070179359167082942891834423311678180448592849484844152837347"
            )
        );
        assert_eq!(
            vm.get_integer((1, 6).into()).unwrap().as_ref(),
            &felt_str!(
                "907662328694455187848008017177970257426839229889571025406355869359245158736"
            )
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_recover_y_hint() {
        let hint_code = hint_code::RECOVER_Y;
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 3;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("x", -3), ("p", -1)];
        // x = 3004956058830981475544150447242655232275382685012344776588097793621230049020
        add_segments!(vm, 2);
        vm.insert_value(
            (1, 0).into(),
            felt_str!(
                "3004956058830981475544150447242655232275382685012344776588097793621230049020"
            ),
        )
        .unwrap();
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        // Check post-hint memory values
        // p.x = 3004956058830981475544150447242655232275382685012344776588097793621230049020
        // p.y = 386236054595386575795345623791920124827519018828430310912260655089307618738
        assert_eq!(
            vm.get_integer((1, 2).into()).unwrap().as_ref(),
            &felt_str!(
                "3004956058830981475544150447242655232275382685012344776588097793621230049020"
            )
        );
        assert_eq!(
            vm.get_integer((1, 3).into()).unwrap().as_ref(),
            &felt_str!(
                "386236054595386575795345623791920124827519018828430310912260655089307618738"
            )
        );
    }
}
