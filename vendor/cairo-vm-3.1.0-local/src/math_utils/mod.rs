mod is_prime;

pub use is_prime::is_prime;

use core::cmp::min;

use crate::stdlib::{boxed::Box, ops::Shr, prelude::Vec};
use crate::types::errors::math_errors::MathError;
use crate::utils::CAIRO_PRIME;
use crate::Felt252;
use lazy_static::lazy_static;
use num_bigint::{BigInt, BigUint, RandBigInt, ToBigInt};
use num_integer::Integer;
use num_traits::{One, Signed, Zero};
use rand::{rngs::SmallRng, SeedableRng};
use starknet_types_core::felt::NonZeroFelt;

lazy_static! {
    pub static ref SIGNED_FELT_MAX: BigUint = (&*CAIRO_PRIME).shr(1_u32);
    static ref POWERS_OF_TWO: Vec<NonZeroFelt> =
        core::iter::successors(Some(Felt252::ONE), |x| Some(x * Felt252::TWO))
            .take(252)
            .map(|x| x.try_into().unwrap())
            .collect::<Vec<_>>();
}

pub const STWO_PRIME: u64 = (1 << 31) - 1;
const STWO_PRIME_U128: u128 = STWO_PRIME as u128;
const MASK_36: u64 = (1 << 36) - 1;
const MASK_8: u64 = (1 << 8) - 1;

/// Returns the `n`th (up to the `251`th power) power of 2 as a [`Felt252`]
/// in constant time.
/// It silently returns `1` if the input is out of bounds.
pub fn pow2_const(n: u32) -> Felt252 {
    // If the conversion fails then it's out of range and we compute the power as usual
    POWERS_OF_TWO
        .get(n as usize)
        .unwrap_or(&POWERS_OF_TWO[0])
        .into()
}

/// Returns the `n`th (up to the `251`th power) power of 2 as a [`&stark_felt::NonZeroFelt`]
/// in constant time.
/// It silently returns `1` if the input is out of bounds.
pub fn pow2_const_nz(n: u32) -> &'static NonZeroFelt {
    // If the conversion fails then it's out of range and we compute the power as usual
    POWERS_OF_TWO.get(n as usize).unwrap_or(&POWERS_OF_TWO[0])
}

/// Converts [`Felt252`] into a [`BigInt`] number in the range: `(- FIELD / 2, FIELD / 2)`.
///
/// # Examples
///
/// ```
/// # use cairo_vm::{Felt252, math_utils::signed_felt};
/// # use num_bigint::BigInt;
/// let positive = Felt252::from(5);
/// assert_eq!(signed_felt(positive), BigInt::from(5));
///
/// let negative = Felt252::MAX;
/// assert_eq!(signed_felt(negative), BigInt::from(-1));
/// ```
pub fn signed_felt(felt: Felt252) -> BigInt {
    let biguint = felt.to_biguint();
    if biguint > *SIGNED_FELT_MAX {
        BigInt::from_biguint(num_bigint::Sign::Minus, &*CAIRO_PRIME - &biguint)
    } else {
        biguint.to_bigint().expect("cannot fail")
    }
}

pub fn signed_felt_for_prime(value: Felt252, prime: &BigUint) -> BigInt {
    let value = value.to_biguint();
    let half_prime = prime / 2u32;
    if value > half_prime {
        BigInt::from_biguint(num_bigint::Sign::Minus, prime - &value)
    } else {
        BigInt::from_biguint(num_bigint::Sign::Plus, value)
    }
}

/// QM31 utility function, used specifically for Stwo.
/// QM31 operations are to be relocated into https://github.com/lambdaclass/lambdaworks.
/// Reads four u64 coordinates from a single Felt252.
/// STWO_PRIME fits in 36 bits, hence each coordinate can be represented by 36 bits and a QM31
/// element can be stored in the first 144 bits of a Felt252.
/// Returns an error if the input has over 144 bits or any coordinate is unreduced.
fn qm31_packed_reduced_read_coordinates(felt: Felt252) -> Result<[u64; 4], MathError> {
    let limbs = felt.to_le_digits();
    if limbs[3] != 0 || limbs[2] >= 1 << 16 {
        return Err(MathError::QM31UnreducedError(Box::new(felt)));
    }
    let coordinates = [
        (limbs[0] & MASK_36),
        ((limbs[0] >> 36) + ((limbs[1] & MASK_8) << 28)),
        ((limbs[1] >> 8) & MASK_36),
        ((limbs[1] >> 44) + (limbs[2] << 20)),
    ];
    for x in coordinates.iter() {
        if *x >= STWO_PRIME {
            return Err(MathError::QM31UnreducedError(Box::new(felt)));
        }
    }
    Ok(coordinates)
}

/// QM31 utility function, used specifically for Stwo.
/// QM31 operations are to be relocated into https://github.com/lambdaclass/lambdaworks.
/// Reduces four u64 coordinates and packs them into a single Felt252.
/// STWO_PRIME fits in 36 bits, hence each coordinate can be represented by 36 bits and a QM31
/// element can be stored in the first 144 bits of a Felt252.
pub(crate) fn qm31_coordinates_to_packed_reduced(coordinates: [u64; 4]) -> Felt252 {
    let bytes_part1 = ((coordinates[0] % STWO_PRIME) as u128
        + (((coordinates[1] % STWO_PRIME) as u128) << 36))
        .to_le_bytes();
    let bytes_part2 = ((coordinates[2] % STWO_PRIME) as u128
        + (((coordinates[3] % STWO_PRIME) as u128) << 36))
        .to_le_bytes();
    let mut result_bytes = [0u8; 32];
    result_bytes[0..9].copy_from_slice(&bytes_part1[0..9]);
    result_bytes[9..18].copy_from_slice(&bytes_part2[0..9]);
    Felt252::from_bytes_le(&result_bytes)
}

/// QM31 utility function, used specifically for Stwo.
/// QM31 operations are to be relocated into https://github.com/lambdaclass/lambdaworks.
/// Computes the addition of two QM31 elements in reduced form.
/// Returns an error if either operand is not reduced.
pub(crate) fn qm31_packed_reduced_add(
    felt1: Felt252,
    felt2: Felt252,
) -> Result<Felt252, MathError> {
    let coordinates1 = qm31_packed_reduced_read_coordinates(felt1)?;
    let coordinates2 = qm31_packed_reduced_read_coordinates(felt2)?;
    let result_unreduced_coordinates = [
        coordinates1[0] + coordinates2[0],
        coordinates1[1] + coordinates2[1],
        coordinates1[2] + coordinates2[2],
        coordinates1[3] + coordinates2[3],
    ];
    Ok(qm31_coordinates_to_packed_reduced(
        result_unreduced_coordinates,
    ))
}

/// QM31 utility function, used specifically for Stwo.
/// QM31 operations are to be relocated into https://github.com/lambdaclass/lambdaworks.
/// Computes the negative of a QM31 element in reduced form.
/// Returns an error if the input is not reduced.
#[allow(dead_code)]
pub(crate) fn qm31_packed_reduced_neg(felt: Felt252) -> Result<Felt252, MathError> {
    let coordinates = qm31_packed_reduced_read_coordinates(felt)?;
    Ok(qm31_coordinates_to_packed_reduced([
        STWO_PRIME - coordinates[0],
        STWO_PRIME - coordinates[1],
        STWO_PRIME - coordinates[2],
        STWO_PRIME - coordinates[3],
    ]))
}

/// QM31 utility function, used specifically for Stwo.
/// QM31 operations are to be relocated into https://github.com/lambdaclass/lambdaworks.
/// Computes the subtraction of two QM31 elements in reduced form.
/// Returns an error if either operand is not reduced.
pub(crate) fn qm31_packed_reduced_sub(
    felt1: Felt252,
    felt2: Felt252,
) -> Result<Felt252, MathError> {
    let coordinates1 = qm31_packed_reduced_read_coordinates(felt1)?;
    let coordinates2 = qm31_packed_reduced_read_coordinates(felt2)?;
    let result_unreduced_coordinates = [
        STWO_PRIME + coordinates1[0] - coordinates2[0],
        STWO_PRIME + coordinates1[1] - coordinates2[1],
        STWO_PRIME + coordinates1[2] - coordinates2[2],
        STWO_PRIME + coordinates1[3] - coordinates2[3],
    ];
    Ok(qm31_coordinates_to_packed_reduced(
        result_unreduced_coordinates,
    ))
}

/// QM31 utility function, used specifically for Stwo.
/// QM31 operations are to be relocated into https://github.com/lambdaclass/lambdaworks.
/// Computes the multiplication of two QM31 elements in reduced form.
/// Returns an error if either operand is not reduced.
pub(crate) fn qm31_packed_reduced_mul(
    felt1: Felt252,
    felt2: Felt252,
) -> Result<Felt252, MathError> {
    let coordinates1_u64 = qm31_packed_reduced_read_coordinates(felt1)?;
    let coordinates2_u64 = qm31_packed_reduced_read_coordinates(felt2)?;
    let coordinates1 = coordinates1_u64.map(u128::from);
    let coordinates2 = coordinates2_u64.map(u128::from);

    let result_coordinates = [
        ((5 * STWO_PRIME_U128 * STWO_PRIME_U128 + coordinates1[0] * coordinates2[0]
            - coordinates1[1] * coordinates2[1]
            + 2 * coordinates1[2] * coordinates2[2]
            - 2 * coordinates1[3] * coordinates2[3]
            - coordinates1[2] * coordinates2[3]
            - coordinates1[3] * coordinates2[2])
            % STWO_PRIME_U128) as u64,
        ((STWO_PRIME_U128 * STWO_PRIME_U128
            + coordinates1[0] * coordinates2[1]
            + coordinates1[1] * coordinates2[0]
            + 2 * (coordinates1[2] * coordinates2[3] + coordinates1[3] * coordinates2[2])
            + coordinates1[2] * coordinates2[2]
            - coordinates1[3] * coordinates2[3])
            % STWO_PRIME_U128) as u64,
        2 * STWO_PRIME * STWO_PRIME + coordinates1_u64[0] * coordinates2_u64[2]
            - coordinates1_u64[1] * coordinates2_u64[3]
            + coordinates1_u64[2] * coordinates2_u64[0]
            - coordinates1_u64[3] * coordinates2_u64[1],
        coordinates1_u64[0] * coordinates2_u64[3]
            + coordinates1_u64[1] * coordinates2_u64[2]
            + coordinates1_u64[2] * coordinates2_u64[1]
            + coordinates1_u64[3] * coordinates2_u64[0],
    ];
    Ok(qm31_coordinates_to_packed_reduced(result_coordinates))
}

/// M31 utility function, used specifically for Stwo.
/// M31 operations are to be relocated into https://github.com/lambdaclass/lambdaworks.
/// Computes the inverse in the M31 field using Fermat's little theorem, i.e., returns
/// `v^(STWO_PRIME-2) modulo STWO_PRIME`, which is the inverse of v unless v % STWO_PRIME == 0.
pub(crate) fn pow2147483645(v: u64) -> u64 {
    let t0 = (sqn(v, 2) * v) % STWO_PRIME;
    let t1 = (sqn(t0, 1) * t0) % STWO_PRIME;
    let t2 = (sqn(t1, 3) * t0) % STWO_PRIME;
    let t3 = (sqn(t2, 1) * t0) % STWO_PRIME;
    let t4 = (sqn(t3, 8) * t3) % STWO_PRIME;
    let t5 = (sqn(t4, 8) * t3) % STWO_PRIME;
    (sqn(t5, 7) * t2) % STWO_PRIME
}

/// M31 utility function, used specifically for Stwo.
/// M31 operations are to be relocated into https://github.com/lambdaclass/lambdaworks.
/// Computes `v^(2^n) modulo STWO_PRIME`.
fn sqn(v: u64, n: usize) -> u64 {
    let mut u = v;
    for _ in 0..n {
        u = (u * u) % STWO_PRIME;
    }
    u
}

/// QM31 utility function, used specifically for Stwo.
/// QM31 operations are to be relocated into https://github.com/lambdaclass/lambdaworks.
/// Computes the inverse of a QM31 element in reduced form.
/// Returns an error if the denominator is zero or either operand is not reduced.
pub(crate) fn qm31_packed_reduced_inv(felt: Felt252) -> Result<Felt252, MathError> {
    if felt.is_zero() {
        return Err(MathError::DividedByZero);
    }
    let coordinates = qm31_packed_reduced_read_coordinates(felt)?;

    let b2_r = (coordinates[2] * coordinates[2] + STWO_PRIME * STWO_PRIME
        - coordinates[3] * coordinates[3])
        % STWO_PRIME;
    let b2_i = (2 * coordinates[2] * coordinates[3]) % STWO_PRIME;

    let denom_r = (coordinates[0] * coordinates[0] + STWO_PRIME * STWO_PRIME
        - coordinates[1] * coordinates[1]
        + 2 * STWO_PRIME
        - 2 * b2_r
        + b2_i)
        % STWO_PRIME;
    let denom_i =
        (2 * coordinates[0] * coordinates[1] + 3 * STWO_PRIME - 2 * b2_i - b2_r) % STWO_PRIME;

    let denom_norm_squared = (denom_r * denom_r + denom_i * denom_i) % STWO_PRIME;
    let denom_norm_inverse_squared = pow2147483645(denom_norm_squared);

    let denom_inverse_r = (denom_r * denom_norm_inverse_squared) % STWO_PRIME;
    let denom_inverse_i = ((STWO_PRIME - denom_i) * denom_norm_inverse_squared) % STWO_PRIME;

    Ok(qm31_coordinates_to_packed_reduced([
        coordinates[0] * denom_inverse_r + STWO_PRIME * STWO_PRIME
            - coordinates[1] * denom_inverse_i,
        coordinates[0] * denom_inverse_i + coordinates[1] * denom_inverse_r,
        coordinates[3] * denom_inverse_i + STWO_PRIME * STWO_PRIME
            - coordinates[2] * denom_inverse_r,
        2 * STWO_PRIME * STWO_PRIME
            - coordinates[2] * denom_inverse_i
            - coordinates[3] * denom_inverse_r,
    ]))
}

/// QM31 utility function, used specifically for Stwo.
/// QM31 operations are to be relocated into https://github.com/lambdaclass/lambdaworks.
/// Computes the division of two QM31 elements in reduced form.
/// Returns an error if the input is zero.
pub(crate) fn qm31_packed_reduced_div(
    felt1: Felt252,
    felt2: Felt252,
) -> Result<Felt252, MathError> {
    let felt2_inv = qm31_packed_reduced_inv(felt2)?;
    qm31_packed_reduced_mul(felt1, felt2_inv)
}

///Returns the integer square root of the nonnegative integer n.
///This is the floor of the exact square root of n.
///Unlike math.sqrt(), this function doesn't have rounding error issues.
pub fn isqrt(n: &BigUint) -> Result<BigUint, MathError> {
    /*    # The following algorithm was copied from
    # https://stackoverflow.com/questions/15390807/integer-square-root-in-python.
    x = n
    y = (x + 1) // 2
    while y < x:
        x = y
        y = (x + n // x) // 2
    assert x**2 <= n < (x + 1) ** 2
    return x*/

    let mut x = n.clone();
    //n.shr(1) = n.div_floor(2)
    let mut y = (&x + 1_u32).shr(1_u32);

    while y < x {
        x = y;
        y = (&x + n.div_floor(&x)).shr(1_u32);
    }

    if !(&BigUint::pow(&x, 2_u32) <= n && n < &BigUint::pow(&(&x + 1_u32), 2_u32)) {
        return Err(MathError::FailedToGetSqrt(Box::new(n.clone())));
    };
    Ok(x)
}

/// Performs integer division between x and y; fails if x is not divisible by y.
pub fn safe_div(x: &Felt252, y: &Felt252) -> Result<Felt252, MathError> {
    let (q, r) = x.div_rem(&y.try_into().map_err(|_| MathError::DividedByZero)?);

    if !r.is_zero() {
        Err(MathError::SafeDivFail(Box::new((*x, *y))))
    } else {
        Ok(q)
    }
}

/// Performs integer division between x and y; fails if x is not divisible by y.
pub fn safe_div_bigint(x: &BigInt, y: &BigInt) -> Result<BigInt, MathError> {
    if y.is_zero() {
        return Err(MathError::DividedByZero);
    }

    let (q, r) = x.div_mod_floor(y);

    if !r.is_zero() {
        return Err(MathError::SafeDivFailBigInt(Box::new((
            x.clone(),
            y.clone(),
        ))));
    }

    Ok(q)
}

/// Performs integer division between x and y; fails if x is not divisible by y.
pub fn safe_div_usize(x: usize, y: usize) -> Result<usize, MathError> {
    if y.is_zero() {
        return Err(MathError::DividedByZero);
    }

    let (q, r) = x.div_mod_floor(&y);

    if !r.is_zero() {
        return Err(MathError::SafeDivFailUsize(Box::new((x, y))));
    }

    Ok(q)
}

///Returns num_a^-1 mod p
pub(crate) fn mul_inv(num_a: &BigInt, p: &BigInt) -> BigInt {
    if num_a.is_zero() {
        return BigInt::zero();
    }
    let mut a = num_a.abs();
    let x_sign = num_a.signum();
    let mut b = p.abs();
    let (mut x, mut r) = (BigInt::one(), BigInt::zero());
    let (mut c, mut q);
    while !b.is_zero() {
        (q, c) = a.div_mod_floor(&b);
        x -= &q * &r;
        (a, b, r, x) = (b, c, x, r)
    }

    x * x_sign
}

///Returns x, y, g such that g = x*a + y*b = gcd(a, b).
fn igcdex(num_a: &BigInt, num_b: &BigInt) -> (BigInt, BigInt, BigInt) {
    match (num_a, num_b) {
        (a, b) if a.is_zero() && b.is_zero() => (BigInt::zero(), BigInt::one(), BigInt::zero()),
        (a, _) if a.is_zero() => (BigInt::zero(), num_b.signum(), num_b.abs()),
        (_, b) if b.is_zero() => (num_a.signum(), BigInt::zero(), num_a.abs()),
        _ => {
            let mut a = num_a.abs();
            let x_sign = num_a.signum();
            let mut b = num_b.abs();
            let y_sign = num_b.signum();
            let (mut x, mut y, mut r, mut s) =
                (BigInt::one(), BigInt::zero(), BigInt::zero(), BigInt::one());
            let (mut c, mut q);
            while !b.is_zero() {
                (q, c) = a.div_mod_floor(&b);
                x -= &q * &r;
                y -= &q * &s;
                (a, b, r, s, x, y) = (b, c, x, y, r, s)
            }
            (x * x_sign, y * y_sign, a)
        }
    }
}

///Finds a nonnegative integer x < p such that (m * x) % p == n.
pub fn div_mod(n: &BigInt, m: &BigInt, p: &BigInt) -> Result<BigInt, MathError> {
    let (a, _, c) = igcdex(m, p);
    if !c.is_one() {
        return Err(MathError::DivModIgcdexNotZero(Box::new((
            n.clone(),
            m.clone(),
            p.clone(),
        ))));
    }
    Ok((n * a).mod_floor(p))
}

pub(crate) fn div_mod_unsigned(
    n: &BigUint,
    m: &BigUint,
    p: &BigUint,
) -> Result<BigUint, MathError> {
    // BigUint to BigInt conversion cannot fail & div_mod will always return a positive value if all values are positive so we can safely unwrap here
    div_mod(
        &n.to_bigint().unwrap(),
        &m.to_bigint().unwrap(),
        &p.to_bigint().unwrap(),
    )
    .map(|i| i.to_biguint().unwrap())
}

pub fn ec_add(
    point_a: (BigInt, BigInt),
    point_b: (BigInt, BigInt),
    prime: &BigInt,
) -> Result<(BigInt, BigInt), MathError> {
    let m = line_slope(&point_a, &point_b, prime)?;
    let x = (m.clone() * m.clone() - point_a.0.clone() - point_b.0).mod_floor(prime);
    let y = (m * (point_a.0 - x.clone()) - point_a.1).mod_floor(prime);
    Ok((x, y))
}

/// Computes the slope of the line connecting the two given EC points over the field GF(p).
/// Assumes the points are given in affine form (x, y) and have different x coordinates.
pub fn line_slope(
    point_a: &(BigInt, BigInt),
    point_b: &(BigInt, BigInt),
    prime: &BigInt,
) -> Result<BigInt, MathError> {
    debug_assert!(!(&point_a.0 - &point_b.0).is_multiple_of(prime));
    div_mod(
        &(&point_a.1 - &point_b.1),
        &(&point_a.0 - &point_b.0),
        prime,
    )
}

///  Doubles a point on an elliptic curve with the equation y^2 = x^3 + alpha*x + beta mod p.
/// Assumes the point is given in affine form (x, y) and has y != 0.
pub fn ec_double(
    point: (BigInt, BigInt),
    alpha: &BigInt,
    prime: &BigInt,
) -> Result<(BigInt, BigInt), MathError> {
    let m = ec_double_slope(&point, alpha, prime)?;
    let x = ((&m * &m) - (2_i32 * &point.0)).mod_floor(prime);
    let y = (m * (point.0 - &x) - point.1).mod_floor(prime);
    Ok((x, y))
}
/// Computes the slope of an elliptic curve with the equation y^2 = x^3 + alpha*x + beta mod p, at
/// the given point.
/// Assumes the point is given in affine form (x, y) and has y != 0.
pub fn ec_double_slope(
    point: &(BigInt, BigInt),
    alpha: &BigInt,
    prime: &BigInt,
) -> Result<BigInt, MathError> {
    debug_assert!(!point.1.is_multiple_of(prime));
    div_mod(
        &(3_i32 * &point.0 * &point.0 + alpha),
        &(2_i32 * &point.1),
        prime,
    )
}

// Adapted from sympy _sqrt_prime_power with k == 1
pub fn sqrt_prime_power(a: &BigUint, p: &BigUint) -> Option<BigUint> {
    if p.is_zero() || !is_prime(p) {
        return None;
    }
    let two = BigUint::from(2_u32);
    let a = a.mod_floor(p);
    if p == &two {
        return Some(a);
    }
    if !(a < two || (a.modpow(&(p - 1_u32).div_floor(&two), p)).is_one()) {
        return None;
    };

    if p.mod_floor(&BigUint::from(4_u32)) == 3_u32.into() {
        let res = a.modpow(&(p + 1_u32).div_floor(&BigUint::from(4_u32)), p);
        return Some(min(res.clone(), p - res));
    };

    if p.mod_floor(&BigUint::from(8_u32)) == 5_u32.into() {
        let sign = a.modpow(&(p - 1_u32).div_floor(&BigUint::from(4_u32)), p);
        if sign.is_one() {
            let res = a.modpow(&(p + 3_u32).div_floor(&BigUint::from(8_u32)), p);
            return Some(min(res.clone(), p - res));
        } else {
            let b = (4_u32 * &a).modpow(&(p - 5_u32).div_floor(&BigUint::from(8_u32)), p);
            let x = (2_u32 * &a * b).mod_floor(p);
            if x.modpow(&two, p) == a {
                return Some(x);
            }
        }
    };

    Some(sqrt_tonelli_shanks(&a, p))
}

fn sqrt_tonelli_shanks(n: &BigUint, prime: &BigUint) -> BigUint {
    // Based on Tonelli-Shanks' algorithm for finding square roots
    // and sympy's library implementation of said algorithm.
    if n.is_zero() || n.is_one() {
        return n.clone();
    }
    let s = (prime - 1_u32).trailing_zeros().unwrap_or_default();
    let t = prime >> s;
    let a = n.modpow(&t, prime);
    // Rng is not critical here so its safe to use a seeded value
    let mut rng = SmallRng::seed_from_u64(11480028852697973135);
    let mut d;
    loop {
        d = RandBigInt::gen_biguint_range(&mut rng, &BigUint::from(2_u32), &(prime - 1_u32));
        let r = legendre_symbol(&d, prime);
        if r == -1 {
            break;
        };
    }
    d = d.modpow(&t, prime);
    let mut m = BigUint::zero();
    let mut exponent = BigUint::one() << (s - 1);
    let mut adm;
    for i in 0..s as u32 {
        adm = &a * &d.modpow(&m, prime);
        adm = adm.modpow(&exponent, prime);
        exponent >>= 1;
        if adm == (prime - 1_u32) {
            m += BigUint::from(1_u32) << i;
        }
    }
    let root_1 =
        (n.modpow(&((t + 1_u32) >> 1), prime) * d.modpow(&(m >> 1), prime)).mod_floor(prime);
    let root_2 = prime - &root_1;
    if root_1 < root_2 {
        root_1
    } else {
        root_2
    }
}

/* Disclaimer: Some asumptions have been taken based on the functions that rely on this function, make sure these are true before calling this function individually
Adpted from sympy implementation, asuming:
    - p is an odd prime number
    - a.mod_floor(p) == a
Returns the Legendre symbol `(a / p)`.

    For an integer ``a`` and an odd prime ``p``, the Legendre symbol is
    defined as

    .. math ::
        \genfrac(){}{}{a}{p} = \begin{cases}
             0 & \text{if } p \text{ divides } a\\
             1 & \text{if } a \text{ is a quadratic residue modulo } p\\
            -1 & \text{if } a \text{ is a quadratic nonresidue modulo } p
        \end{cases}
*/
fn legendre_symbol(a: &BigUint, p: &BigUint) -> i8 {
    if a.is_zero() {
        return 0;
    };
    if is_quad_residue(a, p).unwrap_or_default() {
        1
    } else {
        -1
    }
}

// Ported from sympy implementation
// Simplified as a & p are nonnegative
// Asumes p is a prime number
pub(crate) fn is_quad_residue(a: &BigUint, p: &BigUint) -> Result<bool, MathError> {
    if p.is_zero() {
        return Err(MathError::IsQuadResidueZeroPrime);
    }
    let a = if a >= p { a.mod_floor(p) } else { a.clone() };
    if a < BigUint::from(2_u8) || p < &BigUint::from(3_u8) {
        return Ok(true);
    }
    Ok(
        a.modpow(&(p - BigUint::one()).div_floor(&BigUint::from(2_u8)), p)
            .is_one(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_utils::*;
    use crate::utils::CAIRO_PRIME;
    use assert_matches::assert_matches;

    use num_traits::Num;

    #[cfg(feature = "std")]
    use num_prime::RandPrime;

    #[cfg(feature = "std")]
    use proptest::{array::uniform4, prelude::*};

    // Only used in proptest for now
    #[cfg(feature = "std")]
    use num_bigint::Sign;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_divmod_a() {
        let a = bigint_str!(
            "11260647941622813594563746375280766662237311019551239924981511729608487775604310196863705127454617186486639011517352066501847110680463498585797912894788"
        );
        let b = bigint_str!(
            "4020711254448367604954374443741161860304516084891705811279711044808359405970"
        );
        assert_eq!(
            bigint_str!(
                "2904750555256547440469454488220756360634457312540595732507835416669695939476"
            ),
            div_mod(
                &a,
                &b,
                &BigInt::from_str_radix(&crate::utils::PRIME_STR[2..], 16)
                    .expect("Couldn't parse prime")
            )
            .unwrap()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_divmod_b() {
        let a = bigint_str!(
            "29642372811668969595956851264770043260610851505766181624574941701711520154703788233010819515917136995474951116158286220089597404329949295479559895970988"
        );
        let b = bigint_str!(
            "3443173965374276972000139705137775968422921151703548011275075734291405722262"
        );
        assert_eq!(
            bigint_str!(
                "3601388548860259779932034493250169083811722919049731683411013070523752439691"
            ),
            div_mod(
                &a,
                &b,
                &BigInt::from_str_radix(&crate::utils::PRIME_STR[2..], 16)
                    .expect("Couldn't parse prime")
            )
            .unwrap()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_divmod_c() {
        let a = bigint_str!(
            "1208267356464811040667664150251401430616174694388968865551115897173431833224432165394286799069453655049199580362994484548890574931604445970825506916876"
        );
        let b = bigint_str!(
            "1809792356889571967986805709823554331258072667897598829955472663737669990418"
        );
        assert_eq!(
            bigint_str!(
                "1545825591488572374291664030703937603499513742109806697511239542787093258962"
            ),
            div_mod(
                &a,
                &b,
                &BigInt::from_str_radix(&crate::utils::PRIME_STR[2..], 16)
                    .expect("Couldn't parse prime")
            )
            .unwrap()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_safe_div() {
        let x = Felt252::from(26);
        let y = Felt252::from(13);
        assert_matches!(safe_div(&x, &y), Ok(i) if i == Felt252::from(2));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_safe_div_non_divisor() {
        let x = Felt252::from(25);
        let y = Felt252::from(4);
        let result = safe_div(&x, &y);
        assert_matches!(
            result,
            Err(MathError::SafeDivFail(bx)) if *bx == (Felt252::from(25), Felt252::from(4)));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_safe_div_by_zero() {
        let x = Felt252::from(25);
        let y = Felt252::ZERO;
        let result = safe_div(&x, &y);
        assert_matches!(result, Err(MathError::DividedByZero));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_safe_div_usize() {
        assert_matches!(safe_div_usize(26, 13), Ok(2));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_safe_div_usize_non_divisor() {
        assert_matches!(
            safe_div_usize(25, 4),
            Err(MathError::SafeDivFailUsize(bx)) if *bx == (25, 4)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_safe_div_usize_by_zero() {
        assert_matches!(safe_div_usize(25, 0), Err(MathError::DividedByZero));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_line_slope_for_valid_points() {
        let point_a = (
            bigint_str!(
                "3139037544796708144595053687182055617920475701120786241351436619796497072089"
            ),
            bigint_str!(
                "2119589567875935397690285099786081818522144748339117565577200220779667999801"
            ),
        );
        let point_b = (
            bigint_str!(
                "3324833730090626974525872402899302150520188025637965566623476530814354734325"
            ),
            bigint_str!(
                "3147007486456030910661996439995670279305852583596209647900952752170983517249"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        assert_eq!(
            bigint_str!(
                "992545364708437554384321881954558327331693627531977596999212637460266617010"
            ),
            line_slope(&point_a, &point_b, &prime).unwrap()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_double_slope_for_valid_point_a() {
        let point = (
            bigint_str!(
                "3143372541908290873737380228370996772020829254218248561772745122290262847573"
            ),
            bigint_str!(
                "1721586982687138486000069852568887984211460575851774005637537867145702861131"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        let alpha = bigint!(1);
        assert_eq!(
            bigint_str!(
                "3601388548860259779932034493250169083811722919049731683411013070523752439691"
            ),
            ec_double_slope(&point, &alpha, &prime).unwrap()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_double_slope_for_valid_point_b() {
        let point = (
            bigint_str!(
                "1937407885261715145522756206040455121546447384489085099828343908348117672673"
            ),
            bigint_str!(
                "2010355627224183802477187221870580930152258042445852905639855522404179702985"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        let alpha = bigint!(1);
        assert_eq!(
            bigint_str!(
                "2904750555256547440469454488220756360634457312540595732507835416669695939476"
            ),
            ec_double_slope(&point, &alpha, &prime).unwrap()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_ec_double_for_valid_point_a() {
        let point = (
            bigint_str!(
                "1937407885261715145522756206040455121546447384489085099828343908348117672673"
            ),
            bigint_str!(
                "2010355627224183802477187221870580930152258042445852905639855522404179702985"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        let alpha = bigint!(1);
        assert_eq!(
            (
                bigint_str!(
                    "58460926014232092148191979591712815229424797874927791614218178721848875644"
                ),
                bigint_str!(
                    "1065613861227134732854284722490492186040898336012372352512913425790457998694"
                )
            ),
            ec_double(point, &alpha, &prime).unwrap()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_ec_double_for_valid_point_b() {
        let point = (
            bigint_str!(
                "3143372541908290873737380228370996772020829254218248561772745122290262847573"
            ),
            bigint_str!(
                "1721586982687138486000069852568887984211460575851774005637537867145702861131"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        let alpha = bigint!(1);
        assert_eq!(
            (
                bigint_str!(
                    "1937407885261715145522756206040455121546447384489085099828343908348117672673"
                ),
                bigint_str!(
                    "2010355627224183802477187221870580930152258042445852905639855522404179702985"
                )
            ),
            ec_double(point, &alpha, &prime).unwrap()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_ec_double_for_valid_point_c() {
        let point = (
            bigint_str!(
                "634630432210960355305430036410971013200846091773294855689580772209984122075"
            ),
            bigint_str!(
                "904896178444785983993402854911777165629036333948799414977736331868834995209"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        let alpha = bigint!(1);
        assert_eq!(
            (
                bigint_str!(
                    "3143372541908290873737380228370996772020829254218248561772745122290262847573"
                ),
                bigint_str!(
                    "1721586982687138486000069852568887984211460575851774005637537867145702861131"
                )
            ),
            ec_double(point, &alpha, &prime).unwrap()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_ec_add_for_valid_points_a() {
        let point_a = (
            bigint_str!(
                "1183418161532233795704555250127335895546712857142554564893196731153957537489"
            ),
            bigint_str!(
                "1938007580204102038458825306058547644691739966277761828724036384003180924526"
            ),
        );
        let point_b = (
            bigint_str!(
                "1977703130303461992863803129734853218488251484396280000763960303272760326570"
            ),
            bigint_str!(
                "2565191853811572867032277464238286011368568368717965689023024980325333517459"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        assert_eq!(
            (
                bigint_str!(
                    "1977874238339000383330315148209250828062304908491266318460063803060754089297"
                ),
                bigint_str!(
                    "2969386888251099938335087541720168257053975603483053253007176033556822156706"
                )
            ),
            ec_add(point_a, point_b, &prime).unwrap()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_ec_add_for_valid_points_b() {
        let point_a = (
            bigint_str!(
                "3139037544796708144595053687182055617920475701120786241351436619796497072089"
            ),
            bigint_str!(
                "2119589567875935397690285099786081818522144748339117565577200220779667999801"
            ),
        );
        let point_b = (
            bigint_str!(
                "3324833730090626974525872402899302150520188025637965566623476530814354734325"
            ),
            bigint_str!(
                "3147007486456030910661996439995670279305852583596209647900952752170983517249"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        assert_eq!(
            (
                bigint_str!(
                    "1183418161532233795704555250127335895546712857142554564893196731153957537489"
                ),
                bigint_str!(
                    "1938007580204102038458825306058547644691739966277761828724036384003180924526"
                )
            ),
            ec_add(point_a, point_b, &prime).unwrap()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_ec_add_for_valid_points_c() {
        let point_a = (
            bigint_str!(
                "1183418161532233795704555250127335895546712857142554564893196731153957537489"
            ),
            bigint_str!(
                "1938007580204102038458825306058547644691739966277761828724036384003180924526"
            ),
        );
        let point_b = (
            bigint_str!(
                "1977703130303461992863803129734853218488251484396280000763960303272760326570"
            ),
            bigint_str!(
                "2565191853811572867032277464238286011368568368717965689023024980325333517459"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        assert_eq!(
            (
                bigint_str!(
                    "1977874238339000383330315148209250828062304908491266318460063803060754089297"
                ),
                bigint_str!(
                    "2969386888251099938335087541720168257053975603483053253007176033556822156706"
                )
            ),
            ec_add(point_a, point_b, &prime).unwrap()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_isqrt_a() {
        let n = biguint!(81);
        assert_matches!(isqrt(&n), Ok(x) if x == biguint!(9));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_isqrt_b() {
        let n = biguint_str!("4573659632505831259480");
        assert_matches!(isqrt(&BigUint::pow(&n, 2_u32)), Ok(num) if num == n);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_isqrt_c() {
        let n = biguint_str!(
            "3618502788666131213697322783095070105623107215331596699973092056135872020481"
        );
        assert_matches!(isqrt(&BigUint::pow(&n, 2_u32)), Ok(inner) if inner == n);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_isqrt_zero() {
        let n = BigUint::zero();
        assert_matches!(isqrt(&n), Ok(inner) if inner.is_zero());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn safe_div_bigint_by_zero() {
        let x = BigInt::one();
        let y = BigInt::zero();
        assert_matches!(safe_div_bigint(&x, &y), Err(MathError::DividedByZero))
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_sqrt_prime_power() {
        let n: BigUint = 25_u32.into();
        let p: BigUint = 18446744069414584321_u128.into();
        assert_eq!(sqrt_prime_power(&n, &p), Some(5_u32.into()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_sqrt_prime_power_p_is_zero() {
        let n = BigUint::one();
        let p: BigUint = BigUint::zero();
        assert_eq!(sqrt_prime_power(&n, &p), None);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_sqrt_prime_power_non_prime() {
        let p: BigUint = BigUint::from_bytes_be(&[
            69, 15, 232, 82, 215, 167, 38, 143, 173, 94, 133, 111, 1, 2, 182, 229, 110, 113, 76, 0,
            47, 110, 148, 109, 6, 133, 27, 190, 158, 197, 168, 219, 165, 254, 81, 53, 25, 34,
        ]);
        let n = BigUint::from_bytes_be(&[
            9, 13, 22, 191, 87, 62, 157, 83, 157, 85, 93, 105, 230, 187, 32, 101, 51, 181, 49, 202,
            203, 195, 76, 193, 149, 78, 109, 146, 240, 126, 182, 115, 161, 238, 30, 118, 157, 252,
        ]);

        assert_eq!(sqrt_prime_power(&n, &p), None);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_sqrt_prime_power_none() {
        let n: BigUint = 10_u32.into();
        let p: BigUint = 602_u32.into();
        assert_eq!(sqrt_prime_power(&n, &p), None);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_sqrt_prime_power_prime_two() {
        let n: BigUint = 25_u32.into();
        let p: BigUint = 2_u32.into();
        assert_eq!(sqrt_prime_power(&n, &p), Some(BigUint::one()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_sqrt_prime_power_prime_mod_8_is_5_sign_not_one() {
        let n: BigUint = 676_u32.into();
        let p: BigUint = 9956234341095173_u64.into();
        assert_eq!(
            sqrt_prime_power(&n, &p),
            Some(BigUint::from(9956234341095147_u64))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_sqrt_prime_power_prime_mod_8_is_5_sign_is_one() {
        let n: BigUint = 130283432663_u64.into();
        let p: BigUint = 743900351477_u64.into();
        assert_eq!(
            sqrt_prime_power(&n, &p),
            Some(BigUint::from(123538694848_u64))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_legendre_symbol_zero() {
        assert!(legendre_symbol(&BigUint::zero(), &BigUint::one()).is_zero())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_is_quad_residue_prime_zero() {
        assert_eq!(
            is_quad_residue(&BigUint::one(), &BigUint::zero()),
            Err(MathError::IsQuadResidueZeroPrime)
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_is_quad_residue_prime_a_one_true() {
        assert_eq!(is_quad_residue(&BigUint::one(), &BigUint::one()), Ok(true))
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn mul_inv_0_is_0() {
        let p = &(*CAIRO_PRIME).clone().into();
        let x = &BigInt::zero();
        let x_inv = mul_inv(x, p);

        assert_eq!(x_inv, BigInt::zero());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn igcdex_1_1() {
        assert_eq!(
            igcdex(&BigInt::one(), &BigInt::one()),
            (BigInt::zero(), BigInt::one(), BigInt::one())
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn igcdex_0_0() {
        assert_eq!(
            igcdex(&BigInt::zero(), &BigInt::zero()),
            (BigInt::zero(), BigInt::one(), BigInt::zero())
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn igcdex_1_0() {
        assert_eq!(
            igcdex(&BigInt::one(), &BigInt::zero()),
            (BigInt::one(), BigInt::zero(), BigInt::one())
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn igcdex_4_6() {
        assert_eq!(
            igcdex(&BigInt::from(4), &BigInt::from(6)),
            (BigInt::from(-1), BigInt::one(), BigInt::from(2))
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn qm31_packed_reduced_read_coordinates_over_144_bits() {
        let mut felt_bytes = [0u8; 32];
        felt_bytes[18] = 1;
        let felt = Felt252::from_bytes_le(&felt_bytes);
        assert_matches!(
            qm31_packed_reduced_read_coordinates(felt),
            Err(MathError::QM31UnreducedError(bx)) if *bx == felt
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn qm31_packed_reduced_read_coordinates_unreduced() {
        let mut felt_bytes = [0u8; 32];
        felt_bytes[0] = 0xff;
        felt_bytes[1] = 0xff;
        felt_bytes[2] = 0xff;
        felt_bytes[3] = (1 << 7) - 1;
        let felt = Felt252::from_bytes_le(&felt_bytes);
        assert_matches!(
            qm31_packed_reduced_read_coordinates(felt),
            Err(MathError::QM31UnreducedError(bx)) if *bx == felt
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_qm31_packed_reduced_add() {
        let x_coordinates = [1414213562, 1732050807, 1618033988, 1234567890];
        let y_coordinates = [1234567890, 1414213562, 1732050807, 1618033988];
        let x = qm31_coordinates_to_packed_reduced(x_coordinates);
        let y = qm31_coordinates_to_packed_reduced(y_coordinates);
        let res = qm31_packed_reduced_add(x, y).unwrap();
        let res_coordinates = qm31_packed_reduced_read_coordinates(res);
        assert_eq!(
            res_coordinates,
            Ok([
                (1414213562 + 1234567890) % STWO_PRIME,
                (1732050807 + 1414213562) % STWO_PRIME,
                (1618033988 + 1732050807) % STWO_PRIME,
                (1234567890 + 1618033988) % STWO_PRIME,
            ])
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_qm31_packed_reduced_neg() {
        let x_coordinates = [1749652895, 834624081, 1930174752, 2063872165];
        let x = qm31_coordinates_to_packed_reduced(x_coordinates);
        let res = qm31_packed_reduced_neg(x).unwrap();
        let res_coordinates = qm31_packed_reduced_read_coordinates(res);
        assert_eq!(
            res_coordinates,
            Ok([
                STWO_PRIME - x_coordinates[0],
                STWO_PRIME - x_coordinates[1],
                STWO_PRIME - x_coordinates[2],
                STWO_PRIME - x_coordinates[3]
            ])
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_qm31_packed_reduced_sub() {
        let x_coordinates = [
            (1414213562 + 1234567890) % STWO_PRIME,
            (1732050807 + 1414213562) % STWO_PRIME,
            (1618033988 + 1732050807) % STWO_PRIME,
            (1234567890 + 1618033988) % STWO_PRIME,
        ];
        let y_coordinates = [1414213562, 1732050807, 1618033988, 1234567890];
        let x = qm31_coordinates_to_packed_reduced(x_coordinates);
        let y = qm31_coordinates_to_packed_reduced(y_coordinates);
        let res = qm31_packed_reduced_sub(x, y).unwrap();
        let res_coordinates = qm31_packed_reduced_read_coordinates(res);
        assert_eq!(
            res_coordinates,
            Ok([1234567890, 1414213562, 1732050807, 1618033988])
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_qm31_packed_reduced_mul() {
        let x_coordinates = [1414213562, 1732050807, 1618033988, 1234567890];
        let y_coordinates = [1259921049, 1442249570, 1847759065, 2094551481];
        let x = qm31_coordinates_to_packed_reduced(x_coordinates);
        let y = qm31_coordinates_to_packed_reduced(y_coordinates);
        let res = qm31_packed_reduced_mul(x, y).unwrap();
        let res_coordinates = qm31_packed_reduced_read_coordinates(res);
        assert_eq!(
            res_coordinates,
            Ok([947980980, 1510986506, 623360030, 1260310989])
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_qm31_packed_reduced_inv() {
        let x_coordinates = [1259921049, 1442249570, 1847759065, 2094551481];
        let x = qm31_coordinates_to_packed_reduced(x_coordinates);
        let res = qm31_packed_reduced_inv(x).unwrap();
        assert_eq!(qm31_packed_reduced_mul(x, res), Ok(Felt252::from(1)));

        let x_coordinates = [1, 2, 3, 4];
        let x = qm31_coordinates_to_packed_reduced(x_coordinates);
        let res = qm31_packed_reduced_inv(x).unwrap();
        assert_eq!(qm31_packed_reduced_mul(x, res), Ok(Felt252::from(1)));

        let x_coordinates = [1749652895, 834624081, 1930174752, 2063872165];
        let x = qm31_coordinates_to_packed_reduced(x_coordinates);
        let res = qm31_packed_reduced_inv(x).unwrap();
        assert_eq!(qm31_packed_reduced_mul(x, res), Ok(Felt252::from(1)));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_qm31_packed_reduced_div() {
        let x_coordinates = [1259921049, 1442249570, 1847759065, 2094551481];
        let y_coordinates = [1414213562, 1732050807, 1618033988, 1234567890];
        let xy_coordinates = [947980980, 1510986506, 623360030, 1260310989];
        let x = qm31_coordinates_to_packed_reduced(x_coordinates);
        let y = qm31_coordinates_to_packed_reduced(y_coordinates);
        let xy = qm31_coordinates_to_packed_reduced(xy_coordinates);

        let res = qm31_packed_reduced_div(xy, y).unwrap();
        assert_eq!(res, x);

        let res = qm31_packed_reduced_div(xy, x).unwrap();
        assert_eq!(res, y);
    }

    /// Necessary strat to use proptest on the QM31 test
    #[cfg(feature = "std")]
    fn configuration_strat() -> BoxedStrategy<u64> {
        prop_oneof![Just(0), Just(1), Just(STWO_PRIME - 1), 0..STWO_PRIME].boxed()
    }

    #[cfg(feature = "std")]
    proptest! {

        #[test]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        fn qm31_packed_reduced_inv_random(x_coordinates in uniform4(0u64..STWO_PRIME)
                                                            .prop_filter("All configs cant be 0",
                                                            |arr| !arr.iter().all(|x| *x == 0))
        ) {
            let x = qm31_coordinates_to_packed_reduced(x_coordinates);
            let res = qm31_packed_reduced_inv(x).unwrap();
            assert_eq!(qm31_packed_reduced_mul(x, res), Ok(Felt252::from(1)));
        }

        #[test]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        fn qm31_packed_reduced_inv_extensive(x_coordinates in uniform4(configuration_strat())
                                                            .prop_filter("All configs cant be 0",
                                                            |arr| !arr.iter().all(|x| *x == 0))
                                                            .no_shrink()
        ) {
            let x = qm31_coordinates_to_packed_reduced(x_coordinates);
            let res = qm31_packed_reduced_inv(x).unwrap();
            assert_eq!(qm31_packed_reduced_mul(x, res), Ok(Felt252::from(1)));
        }

        #[test]
        fn pow2_const_in_range_returns_power_of_2(x in 0..=251u32) {
            prop_assert_eq!(pow2_const(x), Felt252::TWO.pow(x));
        }

        #[test]
        fn pow2_const_oob_returns_1(x in 252u32..) {
            prop_assert_eq!(pow2_const(x), Felt252::ONE);
        }

        #[test]
        fn pow2_const_nz_in_range_returns_power_of_2(x in 0..=251u32) {
            prop_assert_eq!(Felt252::from(pow2_const_nz(x)), Felt252::TWO.pow(x));
        }

        #[test]
        fn pow2_const_nz_oob_returns_1(x in 252u32..) {
            prop_assert_eq!(Felt252::from(pow2_const_nz(x)), Felt252::ONE);
        }

        #[test]
        // Test for sqrt_prime_power_ of a quadratic residue. Result should be the minimum root.
        fn sqrt_prime_power_using_random_prime(ref x in any::<[u8; 38]>(), ref y in any::<u64>()) {
            let mut rng = SmallRng::seed_from_u64(*y);
            let x = &BigUint::from_bytes_be(x);
            // Generate a prime here instead of relying on y, otherwise y may never be a prime number
            let p : &BigUint = &RandPrime::gen_prime(&mut rng, 384,  None);
            let x_sq = x * x;
            if let Some(sqrt) = sqrt_prime_power(&x_sq, p) {
                if &sqrt != x {
                    prop_assert_eq!(&(p - sqrt), x);
                } else {
                prop_assert_eq!(&sqrt, x);
                }
            }
        }

        #[test]
        fn mul_inv_x_by_x_is_1(ref x in any::<[u8; 32]>()) {
            let p = &(*CAIRO_PRIME).clone().into();
            let pos_x = &BigInt::from_bytes_be(Sign::Plus, x);
            let neg_x = &BigInt::from_bytes_be(Sign::Minus, x);
            let pos_x_inv = mul_inv(pos_x, p);
            let neg_x_inv = mul_inv(neg_x, p);

            prop_assert_eq!((pos_x * pos_x_inv).mod_floor(p), BigInt::one());
            prop_assert_eq!((neg_x * neg_x_inv).mod_floor(p), BigInt::one());
        }
    }
}
