use indoc::indoc;

use crate::define_hint_string_map;
use crate::stdlib::collections::HashMap;

define_hint_string_map! {
    HINT_CODES,
(ADD_SEGMENT, indoc! {r#"memory[ap] = segments.add()"#}),
(VM_ENTER_SCOPE, indoc! {r#"vm_enter_scope()"#}),
(VM_EXIT_SCOPE, indoc! {r#"vm_exit_scope()"#}),
(MEMCPY_ENTER_SCOPE, indoc! {r#"vm_enter_scope({'n': ids.len})"#}),
(MEMCPY_CONTINUE_COPYING, indoc! {r#"n -= 1
ids.continue_copying = 1 if n > 0 else 0"#}),
(MEMSET_ENTER_SCOPE, indoc! {r#"vm_enter_scope({'n': ids.n})"#}),
(MEMSET_CONTINUE_LOOP, indoc! {r#"n -= 1
ids.continue_loop = 1 if n > 0 else 0"#}),
(POW, indoc! {r#"ids.locs.bit = (ids.prev_locs.exp % PRIME) & 1"#}),
(IS_NN, indoc! {r#"memory[ap] = 0 if 0 <= (ids.a % PRIME) < range_check_builtin.bound else 1"#}),
(IS_NN_OUT_OF_RANGE, indoc! {r#"memory[ap] = 0 if 0 <= ((-ids.a - 1) % PRIME) < range_check_builtin.bound else 1"#}),
(IS_LE_FELT, indoc! {r#"memory[ap] = 0 if (ids.a % PRIME) <= (ids.b % PRIME) else 1"#}),
(IS_POSITIVE, indoc! {r#"from starkware.cairo.common.math_utils import is_positive
ids.is_positive = 1 if is_positive(
    value=ids.value, prime=PRIME, rc_bound=range_check_builtin.bound) else 0"#}),
(ASSERT_NN, indoc! {r#"from starkware.cairo.common.math_utils import assert_integer
assert_integer(ids.a)
assert 0 <= ids.a % PRIME < range_check_builtin.bound, f'a = {ids.a} is out of range.'"#}),
(ASSERT_NOT_ZERO, indoc! {r#"from starkware.cairo.common.math_utils import assert_integer
assert_integer(ids.value)
assert ids.value % PRIME != 0, f'assert_not_zero failed: {ids.value} = 0.'"#}),
(ASSERT_NOT_EQUAL, indoc! {r#"from starkware.cairo.lang.vm.relocatable import RelocatableValue
both_ints = isinstance(ids.a, int) and isinstance(ids.b, int)
both_relocatable = (
    isinstance(ids.a, RelocatableValue) and isinstance(ids.b, RelocatableValue) and
    ids.a.segment_index == ids.b.segment_index)
assert both_ints or both_relocatable, \
    f'assert_not_equal failed: non-comparable values: {ids.a}, {ids.b}.'
assert (ids.a - ids.b) % PRIME != 0, f'assert_not_equal failed: {ids.a} = {ids.b}.'"#}),
(ASSERT_LE_FELT, indoc! {r#"import itertools

from starkware.cairo.common.math_utils import assert_integer
assert_integer(ids.a)
assert_integer(ids.b)
a = ids.a % PRIME
b = ids.b % PRIME
assert a <= b, f'a = {a} is not less than or equal to b = {b}.'

# Find an arc less than PRIME / 3, and another less than PRIME / 2.
lengths_and_indices = [(a, 0), (b - a, 1), (PRIME - 1 - b, 2)]
lengths_and_indices.sort()
assert lengths_and_indices[0][0] <= PRIME // 3 and lengths_and_indices[1][0] <= PRIME // 2
excluded = lengths_and_indices[2][1]

memory[ids.range_check_ptr + 1], memory[ids.range_check_ptr + 0] = (
    divmod(lengths_and_indices[0][0], ids.PRIME_OVER_3_HIGH))
memory[ids.range_check_ptr + 3], memory[ids.range_check_ptr + 2] = (
    divmod(lengths_and_indices[1][0], ids.PRIME_OVER_2_HIGH))"#}),
(ASSERT_LE_FELT_V_0_6, "from starkware.cairo.common.math_utils import assert_integer
assert_integer(ids.a)
assert_integer(ids.b)
assert (ids.a % PRIME) <= (ids.b % PRIME), \\
    f'a = {ids.a % PRIME} is not less than or equal to b = {ids.b % PRIME}.'"),
(ASSERT_LE_FELT_V_0_8, indoc! {r#"from starkware.cairo.common.math_utils import assert_integer
assert_integer(ids.a)
assert_integer(ids.b)
a = ids.a % PRIME
b = ids.b % PRIME
assert a <= b, f'a = {a} is not less than or equal to b = {b}.'

ids.small_inputs = int(
    a < range_check_builtin.bound and (b - a) < range_check_builtin.bound)"#}),
(ASSERT_LE_FELT_EXCLUDED_0, indoc! {r#"memory[ap] = 1 if excluded != 0 else 0"#}),
(ASSERT_LE_FELT_EXCLUDED_1, indoc! {r#"memory[ap] = 1 if excluded != 1 else 0"#}),
(ASSERT_LE_FELT_EXCLUDED_2, indoc! {r#"assert excluded == 2"#}),
(ASSERT_LT_FELT, indoc! {r#"from starkware.cairo.common.math_utils import assert_integer
assert_integer(ids.a)
assert_integer(ids.b)
assert (ids.a % PRIME) < (ids.b % PRIME), \
    f'a = {ids.a % PRIME} is not less than b = {ids.b % PRIME}.'"#}),
(SPLIT_INT_ASSERT_RANGE, indoc! {r#"assert ids.value == 0, 'split_int(): value is out of range.'"#}),
(ASSERT_250_BITS, indoc! {r#"from starkware.cairo.common.math_utils import as_int

# Correctness check.
value = as_int(ids.value, PRIME) % PRIME
assert value < ids.UPPER_BOUND, f'{value} is outside of the range [0, 2**250).'

# Calculation for the assertion.
ids.high, ids.low = divmod(ids.value, ids.SHIFT)"#}),
(IS_250_BITS, indoc! {r#"ids.is_250 = 1 if ids.addr < 2**250 else 0"#}),
(IS_ADDR_BOUNDED, indoc! {r#"# Verify the assumptions on the relationship between 2**250, ADDR_BOUND and PRIME.
ADDR_BOUND = ids.ADDR_BOUND % PRIME
assert (2**250 < ADDR_BOUND <= 2**251) and (2 * 2**250 < PRIME) and (
        ADDR_BOUND * 2 > PRIME), \
    'normalize_address() cannot be used with the current constants.'
ids.is_small = 1 if ids.addr < ADDR_BOUND else 0"#}),
(SPLIT_INT, indoc! {r#"memory[ids.output] = res = (int(ids.value) % PRIME) % ids.base
assert res < ids.bound, f'split_int(): Limb {res} is out of range.'"#}),
(SPLIT_64, indoc! {r#"ids.low = ids.a & ((1<<64) - 1)
ids.high = ids.a >> 64"#}),
(SPLIT_FELT, indoc! {r#"from starkware.cairo.common.math_utils import assert_integer
assert ids.MAX_HIGH < 2**128 and ids.MAX_LOW < 2**128
assert PRIME - 1 == ids.MAX_HIGH * 2**128 + ids.MAX_LOW
assert_integer(ids.value)
ids.low = ids.value & ((1 << 128) - 1)
ids.high = ids.value >> 128"#}),
(SQRT, indoc! {r#"from starkware.python.math_utils import isqrt
value = ids.value % PRIME
assert value < 2 ** 250, f"value={value} is outside of the range [0, 2**250)."
assert 2 ** 250 < PRIME
ids.root = isqrt(value)"#}),
(UNSIGNED_DIV_REM, indoc! {r#"from starkware.cairo.common.math_utils import assert_integer
assert_integer(ids.div)
assert 0 < ids.div <= PRIME // range_check_builtin.bound, \
    f'div={hex(ids.div)} is out of the valid range.'
ids.q, ids.r = divmod(ids.value, ids.div)"#}),
(SIGNED_DIV_REM, indoc! {r#"from starkware.cairo.common.math_utils import as_int, assert_integer

assert_integer(ids.div)
assert 0 < ids.div <= PRIME // range_check_builtin.bound, \
    f'div={hex(ids.div)} is out of the valid range.'

assert_integer(ids.bound)
assert ids.bound <= range_check_builtin.bound // 2, \
    f'bound={hex(ids.bound)} is out of the valid range.'

int_value = as_int(ids.value, PRIME)
q, ids.r = divmod(int_value, ids.div)

assert -ids.bound <= q < ids.bound, \
    f'{int_value} / {ids.div} = {q} is out of the range [{-ids.bound}, {ids.bound}).'

ids.biased_q = q + ids.bound"#}),
(IS_QUAD_RESIDUE, indoc! {r#"from starkware.crypto.signature.signature import FIELD_PRIME
from starkware.python.math_utils import div_mod, is_quad_residue, sqrt

x = ids.x
if is_quad_residue(x, FIELD_PRIME):
    ids.y = sqrt(x, FIELD_PRIME)
else:
    ids.y = sqrt(div_mod(x, 3, FIELD_PRIME), FIELD_PRIME)"#}),
(FIND_ELEMENT, indoc! {r#"array_ptr = ids.array_ptr
elm_size = ids.elm_size
assert isinstance(elm_size, int) and elm_size > 0, \
    f'Invalid value for elm_size. Got: {elm_size}.'
key = ids.key

if '__find_element_index' in globals():
    ids.index = __find_element_index
    found_key = memory[array_ptr + elm_size * __find_element_index]
    assert found_key == key, \
        f'Invalid index found in __find_element_index. index: {__find_element_index}, ' \
        f'expected key {key}, found key: {found_key}.'
    # Delete __find_element_index to make sure it's not used for the next calls.
    del __find_element_index
else:
    n_elms = ids.n_elms
    assert isinstance(n_elms, int) and n_elms >= 0, \
        f'Invalid value for n_elms. Got: {n_elms}.'
    if '__find_element_max_size' in globals():
        assert n_elms <= __find_element_max_size, \
            f'find_element() can only be used with n_elms<={__find_element_max_size}. ' \
            f'Got: n_elms={n_elms}.'

    for i in range(n_elms):
        if memory[array_ptr + elm_size * i] == key:
            ids.index = i
            break
    else:
        raise ValueError(f'Key {key} was not found.')"#}),
(SEARCH_SORTED_LOWER, indoc! {r#"array_ptr = ids.array_ptr
elm_size = ids.elm_size
assert isinstance(elm_size, int) and elm_size > 0, \
    f'Invalid value for elm_size. Got: {elm_size}.'

n_elms = ids.n_elms
assert isinstance(n_elms, int) and n_elms >= 0, \
    f'Invalid value for n_elms. Got: {n_elms}.'
if '__find_element_max_size' in globals():
    assert n_elms <= __find_element_max_size, \
        f'find_element() can only be used with n_elms<={__find_element_max_size}. ' \
        f'Got: n_elms={n_elms}.'

for i in range(n_elms):
    if memory[array_ptr + elm_size * i] >= ids.key:
        ids.index = i
        break
else:
    ids.index = n_elms"#}),
(SET_ADD, indoc! {r#"assert ids.elm_size > 0
assert ids.set_ptr <= ids.set_end_ptr
elm_list = memory.get_range(ids.elm_ptr, ids.elm_size)
for i in range(0, ids.set_end_ptr - ids.set_ptr, ids.elm_size):
    if memory.get_range(ids.set_ptr + i, ids.elm_size) == elm_list:
        ids.index = i // ids.elm_size
        ids.is_elm_in_set = 1
        break
else:
    ids.is_elm_in_set = 0"#}),
(DEFAULT_DICT_NEW, indoc! {r#"if '__dict_manager' not in globals():
    from starkware.cairo.common.dict import DictManager
    __dict_manager = DictManager()

memory[ap] = __dict_manager.new_default_dict(segments, ids.default_value)"#}),
(DICT_NEW, indoc! {r#"if '__dict_manager' not in globals():
    from starkware.cairo.common.dict import DictManager
    __dict_manager = DictManager()

memory[ap] = __dict_manager.new_dict(segments, initial_dict)
del initial_dict"#}),
(DICT_READ, indoc! {r#"dict_tracker = __dict_manager.get_tracker(ids.dict_ptr)
dict_tracker.current_ptr += ids.DictAccess.SIZE
ids.value = dict_tracker.data[ids.key]"#}),
(DICT_WRITE, indoc! {r#"dict_tracker = __dict_manager.get_tracker(ids.dict_ptr)
dict_tracker.current_ptr += ids.DictAccess.SIZE
ids.dict_ptr.prev_value = dict_tracker.data[ids.key]
dict_tracker.data[ids.key] = ids.new_value"#}),
(DICT_UPDATE, indoc! {r#"# Verify dict pointer and prev value.
dict_tracker = __dict_manager.get_tracker(ids.dict_ptr)
current_value = dict_tracker.data[ids.key]
assert current_value == ids.prev_value, \
    f'Wrong previous value in dict. Got {ids.prev_value}, expected {current_value}.'

# Update value.
dict_tracker.data[ids.key] = ids.new_value
dict_tracker.current_ptr += ids.DictAccess.SIZE"#}),
(SQUASH_DICT, indoc! {r#"dict_access_size = ids.DictAccess.SIZE
address = ids.dict_accesses.address_
assert ids.ptr_diff % dict_access_size == 0, \
    'Accesses array size must be divisible by DictAccess.SIZE'
n_accesses = ids.n_accesses
if '__squash_dict_max_size' in globals():
    assert n_accesses <= __squash_dict_max_size, \
        f'squash_dict() can only be used with n_accesses<={__squash_dict_max_size}. ' \
        f'Got: n_accesses={n_accesses}.'
# A map from key to the list of indices accessing it.
access_indices = {}
for i in range(n_accesses):
    key = memory[address + dict_access_size * i]
    access_indices.setdefault(key, []).append(i)
# Descending list of keys.
keys = sorted(access_indices.keys(), reverse=True)
# Are the keys used bigger than range_check bound.
ids.big_keys = 1 if keys[0] >= range_check_builtin.bound else 0
ids.first_key = key = keys.pop()"#}),
(SQUASH_DICT_INNER_SKIP_LOOP, indoc! {r#"ids.should_skip_loop = 0 if current_access_indices else 1"#}),
(SQUASH_DICT_INNER_FIRST_ITERATION, indoc! {r#"current_access_indices = sorted(access_indices[key])[::-1]
current_access_index = current_access_indices.pop()
memory[ids.range_check_ptr] = current_access_index"#}),
(SQUASH_DICT_INNER_CHECK_ACCESS_INDEX, indoc! {r#"new_access_index = current_access_indices.pop()
ids.loop_temps.index_delta_minus1 = new_access_index - current_access_index - 1
current_access_index = new_access_index"#}),
(SQUASH_DICT_INNER_CONTINUE_LOOP, indoc! {r#"ids.loop_temps.should_continue = 1 if current_access_indices else 0"#}),
(SQUASH_DICT_INNER_ASSERT_LEN_KEYS, indoc! {r#"assert len(keys) == 0"#}),
(SQUASH_DICT_INNER_LEN_ASSERT, indoc! {r#"assert len(current_access_indices) == 0"#}),
(SQUASH_DICT_INNER_USED_ACCESSES_ASSERT, indoc! {r#"assert ids.n_used_accesses == len(access_indices[key])"#}),
(SQUASH_DICT_INNER_NEXT_KEY, indoc! {r#"assert len(keys) > 0, 'No keys left but remaining_accesses > 0.'
ids.next_key = key = keys.pop()"#}),
(DICT_SQUASH_COPY_DICT, indoc! {r#"# Prepare arguments for dict_new. In particular, the same dictionary values should be copied
# to the new (squashed) dictionary.
vm_enter_scope({
    # Make __dict_manager accessible.
    '__dict_manager': __dict_manager,
    # Create a copy of the dict, in case it changes in the future.
    'initial_dict': dict(__dict_manager.get_dict(ids.dict_accesses_end)),
})"#}),
(DICT_SQUASH_UPDATE_PTR, indoc! {r#"# Update the DictTracker's current_ptr to point to the end of the squashed dict.
__dict_manager.get_tracker(ids.squashed_dict_start).current_ptr = \
    ids.squashed_dict_end.address_"#}),
(BIGINT_TO_UINT256, indoc! {r#"ids.low = (ids.x.d0 + ids.x.d1 * ids.BASE) & ((1 << 128) - 1)"#}),
(UINT256_ADD, indoc! {r#"sum_low = ids.a.low + ids.b.low
ids.carry_low = 1 if sum_low >= ids.SHIFT else 0
sum_high = ids.a.high + ids.b.high + ids.carry_low
ids.carry_high = 1 if sum_high >= ids.SHIFT else 0"#}),
(UINT256_ADD_LOW, indoc! {r#"sum_low = ids.a.low + ids.b.low
ids.carry_low = 1 if sum_low >= ids.SHIFT else 0"#}),
(UINT128_ADD, indoc! {r#"res = ids.a + ids.b
ids.carry = 1 if res >= ids.SHIFT else 0"#}),
(UINT256_SUB, indoc! {r#"def split(num: int, num_bits_shift: int = 128, length: int = 2):
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
ids.res.high = res_split[1]"#}),
(UINT256_SQRT, indoc! {r#"from starkware.python.math_utils import isqrt
n = (ids.n.high << 128) + ids.n.low
root = isqrt(n)
assert 0 <= root < 2 ** 128
ids.root.low = root
ids.root.high = 0"#}),
(UINT256_SQRT_FELT, indoc! {r#"from starkware.python.math_utils import isqrt
n = (ids.n.high << 128) + ids.n.low
root = isqrt(n)
assert 0 <= root < 2 ** 128
ids.root = root"#}),
(UINT256_SIGNED_NN, indoc! {r#"memory[ap] = 1 if 0 <= (ids.a.high % PRIME) < 2 ** 127 else 0"#}),
(UINT256_UNSIGNED_DIV_REM, indoc! {r#"a = (ids.a.high << 128) + ids.a.low
div = (ids.div.high << 128) + ids.div.low
quotient, remainder = divmod(a, div)

ids.quotient.low = quotient & ((1 << 128) - 1)
ids.quotient.high = quotient >> 128
ids.remainder.low = remainder & ((1 << 128) - 1)
ids.remainder.high = remainder >> 128"#}),
(UINT256_EXPANDED_UNSIGNED_DIV_REM, indoc! {r#"a = (ids.a.high << 128) + ids.a.low
div = (ids.div.b23 << 128) + ids.div.b01
quotient, remainder = divmod(a, div)

ids.quotient.low = quotient & ((1 << 128) - 1)
ids.quotient.high = quotient >> 128
ids.remainder.low = remainder & ((1 << 128) - 1)
ids.remainder.high = remainder >> 128"#}),
(UINT256_MUL_DIV_MOD, indoc! {r#"a = (ids.a.high << 128) + ids.a.low
b = (ids.b.high << 128) + ids.b.low
div = (ids.div.high << 128) + ids.div.low
quotient, remainder = divmod(a * b, div)

ids.quotient_low.low = quotient & ((1 << 128) - 1)
ids.quotient_low.high = (quotient >> 128) & ((1 << 128) - 1)
ids.quotient_high.low = (quotient >> 256) & ((1 << 128) - 1)
ids.quotient_high.high = quotient >> 384
ids.remainder.low = remainder & ((1 << 128) - 1)
ids.remainder.high = remainder >> 128"#}),
(USORT_ENTER_SCOPE, indoc! {r#"vm_enter_scope(dict(__usort_max_size = globals().get('__usort_max_size')))"#}),
(USORT_BODY, indoc! {r#"from collections import defaultdict

input_ptr = ids.input
input_len = int(ids.input_len)
if __usort_max_size is not None:
    assert input_len <= __usort_max_size, (
        f"usort() can only be used with input_len<={__usort_max_size}. "
        f"Got: input_len={input_len}."
    )

positions_dict = defaultdict(list)
for i in range(input_len):
    val = memory[input_ptr + i]
    positions_dict[val].append(i)

output = sorted(positions_dict.keys())
ids.output_len = len(output)
ids.output = segments.gen_arg(output)
ids.multiplicities = segments.gen_arg([len(positions_dict[k]) for k in output])"#}),
(USORT_VERIFY, indoc! {r#"last_pos = 0
positions = positions_dict[ids.value][::-1]"#}),
(USORT_VERIFY_MULTIPLICITY_ASSERT, indoc! {r#"assert len(positions) == 0"#}),
(USORT_VERIFY_MULTIPLICITY_BODY, indoc! {r#"current_pos = positions.pop()
ids.next_item_index = current_pos - last_pos
last_pos = current_pos + 1"#}),
(BLAKE2S_COMPUTE, indoc! {r#"from starkware.cairo.common.cairo_blake2s.blake2s_utils import compute_blake2s_func
compute_blake2s_func(segments=segments, output_ptr=ids.output)"#}),
(BLAKE2S_FINALIZE, indoc! {r#"# Add dummy pairs of input and output.
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
segments.write_arg(ids.blake2s_ptr_end, padding)"#}),
(BLAKE2S_FINALIZE_V2, indoc! {r#"# Add dummy pairs of input and output.
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
padding = (modified_iv + message + [0, 0xffffffff] + output) * (_n_packed_instances - 1)
segments.write_arg(ids.blake2s_ptr_end, padding)"#}),
(BLAKE2S_FINALIZE_V3, indoc! {r#"# Add dummy pairs of input and output.
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
segments.write_arg(ids.blake2s_ptr_end, padding)"#}),
(BLAKE2S_ADD_UINT256, indoc! {r#"B = 32
MASK = 2 ** 32 - 1
segments.write_arg(ids.data, [(ids.low >> (B * i)) & MASK for i in range(4)])
segments.write_arg(ids.data + 4, [(ids.high >> (B * i)) & MASK for i in range(4)])"#}),
(BLAKE2S_ADD_UINT256_BIGEND, indoc! {r#"B = 32
MASK = 2 ** 32 - 1
segments.write_arg(ids.data, [(ids.high >> (B * (3 - i))) & MASK for i in range(4)])
segments.write_arg(ids.data + 4, [(ids.low >> (B * (3 - i))) & MASK for i in range(4)])"#}),
(IS_LESS_THAN_63_BITS_AND_NOT_END, indoc! {r#"memory[ap] = to_felt_or_relocatable((ids.end != ids.packed_values) and (memory[ids.packed_values] < 2**63))"#}),
(BLAKE2S_UNPACK_FELTS, indoc! {r#"offset = 0
for i in range(ids.packed_values_len):
    val = (memory[ids.packed_values + i] % PRIME)
    val_len = 2 if val < 2**63 else 8
    if val_len == 8:
        val += 2**255
    for i in range(val_len - 1, -1, -1):
        val, memory[ids.unpacked_u32s + offset + i] = divmod(val, 2**32)
    assert val == 0
    offset += val_len"#}),
(EXAMPLE_BLAKE2S_COMPRESS, indoc! {r#"from starkware.cairo.common.cairo_blake2s.blake2s_utils import IV, blake2s_compress

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

segments.write_arg(ids.output, new_state)"#}),
(NONDET_BIGINT3_V1, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import split

segments.write_arg(ids.res.address_, split(value))"#}),
(NONDET_BIGINT3_V2, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import split
segments.write_arg(ids.res.address_, split(value))"#}),
(VERIFY_ZERO_V1, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

q, r = divmod(pack(ids.val, PRIME), SECP_P)
assert r == 0, f"verify_zero: Invalid input {ids.val.d0, ids.val.d1, ids.val.d2}."
ids.q = q % PRIME"#}),
(VERIFY_ZERO_V2, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P
q, r = divmod(pack(ids.val, PRIME), SECP_P)
assert r == 0, f"verify_zero: Invalid input {ids.val.d0, ids.val.d1, ids.val.d2}."
ids.q = q % PRIME"#}),
(VERIFY_ZERO_V3, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
SECP_P = 2**255-19
to_assert = pack(ids.val, PRIME)
q, r = divmod(pack(ids.val, PRIME), SECP_P)
assert r == 0, f"verify_zero: Invalid input {ids.val.d0, ids.val.d1, ids.val.d2}."
ids.q = q % PRIME"#}),
(VERIFY_ZERO_EXTERNAL_SECP, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack

q, r = divmod(pack(ids.val, PRIME), SECP_P)
assert r == 0, f"verify_zero: Invalid input {ids.val.d0, ids.val.d1, ids.val.d2}."
ids.q = q % PRIME"#}),
(REDUCE_V1, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

value = pack(ids.x, PRIME) % SECP_P"#}),
(REDUCE_V2, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
value = pack(ids.x, PRIME) % SECP_P"#}),
(REDUCE_ED25519, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
SECP_P=2**255-19

value = pack(ids.x, PRIME) % SECP_P"#}),
(UNSAFE_KECCAK, indoc! {r#"from eth_hash.auto import keccak

data, length = ids.data, ids.length

if '__keccak_max_size' in globals():
    assert length <= __keccak_max_size, \
        f'unsafe_keccak() can only be used with length<={__keccak_max_size}. ' \
        f'Got: length={length}.'

keccak_input = bytearray()
for word_i, byte_i in enumerate(range(0, length, 16)):
    word = memory[data + word_i]
    n_bytes = min(16, length - byte_i)
    assert 0 <= word < 2 ** (8 * n_bytes)
    keccak_input += word.to_bytes(n_bytes, 'big')

hashed = keccak(keccak_input)
ids.high = int.from_bytes(hashed[:16], 'big')
ids.low = int.from_bytes(hashed[16:32], 'big')"#}),
(UNSAFE_KECCAK_FINALIZE, indoc! {r#"from eth_hash.auto import keccak
keccak_input = bytearray()
n_elms = ids.keccak_state.end_ptr - ids.keccak_state.start_ptr
for word in memory.get_range(ids.keccak_state.start_ptr, n_elms):
    keccak_input += word.to_bytes(16, 'big')
hashed = keccak(keccak_input)
ids.high = int.from_bytes(hashed[:16], 'big')
ids.low = int.from_bytes(hashed[16:32], 'big')"#}),
(IS_ZERO_NONDET, indoc! {r#"memory[ap] = to_felt_or_relocatable(x == 0)"#}),
(IS_ZERO_INT, indoc! {r#"memory[ap] = int(x == 0)"#}),
(IS_ZERO_PACK_V1, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

x = pack(ids.x, PRIME) % SECP_P"#}),
(IS_ZERO_PACK_V2, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack
x = pack(ids.x, PRIME) % SECP_P"#}),
(IS_ZERO_PACK_EXTERNAL_SECP_V1, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack

x = pack(ids.x, PRIME) % SECP_P"#}),
(IS_ZERO_PACK_EXTERNAL_SECP_V2, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
x = pack(ids.x, PRIME) % SECP_P"#}),
(IS_ZERO_PACK_ED25519, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
SECP_P=2**255-19

x = pack(ids.x, PRIME) % SECP_P"#}),
(IS_ZERO_ASSIGN_SCOPE_VARS, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P
from starkware.python.math_utils import div_mod

value = x_inv = div_mod(1, x, SECP_P)"#}),
(IS_ZERO_ASSIGN_SCOPE_VARS_EXTERNAL_SECP, indoc! {r#"from starkware.python.math_utils import div_mod

value = x_inv = div_mod(1, x, SECP_P)"#}),
(IS_ZERO_ASSIGN_SCOPE_VARS_ED25519, indoc! {r#"SECP_P=2**255-19
from starkware.python.math_utils import div_mod

value = x_inv = div_mod(1, x, SECP_P)"#}),
(DIV_MOD_N_PACKED_DIVMOD_V1, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import N, pack
from starkware.python.math_utils import div_mod, safe_div

a = pack(ids.a, PRIME)
b = pack(ids.b, PRIME)
value = res = div_mod(a, b, N)"#}),
(DIV_MOD_N_PACKED_DIVMOD_EXTERNAL_N, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
from starkware.python.math_utils import div_mod, safe_div

a = pack(ids.a, PRIME)
b = pack(ids.b, PRIME)
value = res = div_mod(a, b, N)"#}),
(DIV_MOD_N_SAFE_DIV, indoc! {r#"value = k = safe_div(res * b - a, N)"#}),
(GET_FELT_BIT_LENGTH, indoc! {r#"x = ids.x
ids.bit_length = x.bit_length()"#}),
(BIGINT_PACK_DIV_MOD, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
from starkware.cairo.common.math_utils import as_int
from starkware.python.math_utils import div_mod, safe_div

p = pack(ids.P, PRIME)
x = pack(ids.x, PRIME) + as_int(ids.x.d3, PRIME) * ids.BASE ** 3 + as_int(ids.x.d4, PRIME) * ids.BASE ** 4
y = pack(ids.y, PRIME)

value = res = div_mod(x, y, p)"#}),
(BIGINT_SAFE_DIV, indoc! {r#"k = safe_div(res * y - x, p)
value = k if k > 0 else 0 - k
ids.flag = 1 if k > 0 else 0"#}),
(DIV_MOD_N_SAFE_DIV_PLUS_ONE, indoc! {r#"value = k_plus_one = safe_div(res * b - a, N) + 1"#}),
(GET_POINT_FROM_X, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

x_cube_int = pack(ids.x_cube, PRIME) % SECP_P
y_square_int = (x_cube_int + ids.BETA) % SECP_P
y = pow(y_square_int, (SECP_P + 1) // 4, SECP_P)

# We need to decide whether to take y or SECP_P - y.
if ids.v % 2 == y % 2:
    value = y
else:
    value = (-y) % SECP_P"#}),
(EC_NEGATE, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

y = pack(ids.point.y, PRIME) % SECP_P
# The modulo operation in python always returns a nonnegative number.
value = (-y) % SECP_P"#}),
(EC_NEGATE_EMBEDDED_SECP, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
SECP_P = 2**255-19

y = pack(ids.point.y, PRIME) % SECP_P
# The modulo operation in python always returns a nonnegative number.
value = (-y) % SECP_P"#}),
(EC_DOUBLE_SLOPE_V1, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack
from starkware.python.math_utils import ec_double_slope

# Compute the slope.
x = pack(ids.point.x, PRIME)
y = pack(ids.point.y, PRIME)
value = slope = ec_double_slope(point=(x, y), alpha=0, p=SECP_P)"#}),
(EC_DOUBLE_SLOPE_V2, indoc! {r#"from starkware.python.math_utils import ec_double_slope
from starkware.cairo.common.cairo_secp.secp_utils import pack
SECP_P = 2**255-19

# Compute the slope.
x = pack(ids.point.x, PRIME)
y = pack(ids.point.y, PRIME)
value = slope = ec_double_slope(point=(x, y), alpha=42204101795669822316448953119945047945709099015225996174933988943478124189485, p=SECP_P)"#}),
(EC_DOUBLE_SLOPE_V3, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack
from starkware.python.math_utils import div_mod

# Compute the slope.
x = pack(ids.pt.x, PRIME)
y = pack(ids.pt.y, PRIME)
value = slope = div_mod(3 * x ** 2, 2 * y, SECP_P)"#}),
(EC_DOUBLE_SLOPE_V4, indoc! {r#"from starkware.cairo.common.cairo_secp.secp256r1_utils import SECP256R1_ALPHA, SECP256R1_P
from starkware.cairo.common.cairo_secp.secp_utils import pack
from starkware.python.math_utils import ec_double_slope

# Compute the slope.
x = pack(ids.point.x, SECP256R1_P)
y = pack(ids.point.y, SECP256R1_P)
value = slope = ec_double_slope(point=(x, y), alpha=SECP256R1_ALPHA, p=SECP256R1_P)"#}),
(EC_DOUBLE_SLOPE_V5, indoc! {r#"from starkware.cairo.common.cairo_secp.secp256r1_utils import SECP256R1_ALPHA, SECP256R1_P
from starkware.cairo.common.cairo_secp.secp_utils import pack
from starkware.python.math_utils import ec_double_slope

# Compute the slope.
x = pack(ids.point.x, PRIME)
y = pack(ids.point.y, PRIME)
value = slope = ec_double_slope(point=(x, y), alpha=SECP256R1_ALPHA, p=SECP256R1_P)"#}),
(EC_DOUBLE_SLOPE_EXTERNAL_CONSTS, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
from starkware.python.math_utils import ec_double_slope

# Compute the slope.
x = pack(ids.point.x, PRIME)
y = pack(ids.point.y, PRIME)
value = slope = ec_double_slope(point=(x, y), alpha=ALPHA, p=SECP_P)"#}),
(COMPUTE_SLOPE_V1, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack
from starkware.python.math_utils import line_slope

# Compute the slope.
x0 = pack(ids.point0.x, PRIME)
y0 = pack(ids.point0.y, PRIME)
x1 = pack(ids.point1.x, PRIME)
y1 = pack(ids.point1.y, PRIME)
value = slope = line_slope(point1=(x0, y0), point2=(x1, y1), p=SECP_P)"#}),
(COMPUTE_SLOPE_V2, indoc! {r#"from starkware.python.math_utils import line_slope
from starkware.cairo.common.cairo_secp.secp_utils import pack
SECP_P = 2**255-19
# Compute the slope.
x0 = pack(ids.point0.x, PRIME)
y0 = pack(ids.point0.y, PRIME)
x1 = pack(ids.point1.x, PRIME)
y1 = pack(ids.point1.y, PRIME)
value = slope = line_slope(point1=(x0, y0), point2=(x1, y1), p=SECP_P)"#}),
(COMPUTE_SLOPE_SECP256R1_V1, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
from starkware.python.math_utils import line_slope

# Compute the slope.
x0 = pack(ids.point0.x, PRIME)
y0 = pack(ids.point0.y, PRIME)
x1 = pack(ids.point1.x, PRIME)
y1 = pack(ids.point1.y, PRIME)
value = slope = line_slope(point1=(x0, y0), point2=(x1, y1), p=SECP_P)"#}),
(COMPUTE_SLOPE_SECP256R1_V2, indoc! {r#"from starkware.cairo.common.cairo_secp.secp256r1_utils import SECP256R1_P
from starkware.cairo.common.cairo_secp.secp_utils import pack
from starkware.python.math_utils import line_slope

# Compute the slope.
x0 = pack(ids.point0.x, PRIME)
y0 = pack(ids.point0.y, PRIME)
x1 = pack(ids.point1.x, PRIME)
y1 = pack(ids.point1.y, PRIME)
value = slope = line_slope(point1=(x0, y0), point2=(x1, y1), p=SECP256R1_P)"#}),
(IMPORT_SECP256R1_P, indoc! {r#"from starkware.cairo.common.cairo_secp.secp256r1_utils import SECP256R1_P as SECP_P"#}),
(COMPUTE_SLOPE_WHITELIST, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack
from starkware.python.math_utils import div_mod

# Compute the slope.
x0 = pack(ids.pt0.x, PRIME)
y0 = pack(ids.pt0.y, PRIME)
x1 = pack(ids.pt1.x, PRIME)
y1 = pack(ids.pt1.y, PRIME)
value = slope = div_mod(y0 - y1, x0 - x1, SECP_P)"#}),
(EC_DOUBLE_ASSIGN_NEW_X_V1, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

slope = pack(ids.slope, PRIME)
x = pack(ids.point.x, PRIME)
y = pack(ids.point.y, PRIME)

value = new_x = (pow(slope, 2, SECP_P) - 2 * x) % SECP_P"#}),
(EC_DOUBLE_ASSIGN_NEW_X_V2, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack

slope = pack(ids.slope, PRIME)
x = pack(ids.point.x, PRIME)
y = pack(ids.point.y, PRIME)

value = new_x = (pow(slope, 2, SECP_P) - 2 * x) % SECP_P"#}),
(EC_DOUBLE_ASSIGN_NEW_X_V3, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
SECP_P = 2**255-19

slope = pack(ids.slope, PRIME)
x = pack(ids.point.x, PRIME)
y = pack(ids.point.y, PRIME)

value = new_x = (pow(slope, 2, SECP_P) - 2 * x) % SECP_P"#}),
(EC_DOUBLE_ASSIGN_NEW_X_V4, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

slope = pack(ids.slope, PRIME)
x = pack(ids.pt.x, PRIME)
y = pack(ids.pt.y, PRIME)

value = new_x = (pow(slope, 2, SECP_P) - 2 * x) % SECP_P"#}),
(EC_DOUBLE_ASSIGN_NEW_Y, indoc! {r#"value = new_y = (slope * (x - new_x) - y) % SECP_P"#}),
(SHA256_INPUT, indoc! {r#"ids.full_word = int(ids.n_bytes >= 4)"#}),
(SHA256_MAIN_CONSTANT_INPUT_LENGTH, indoc! {r#"from starkware.cairo.common.cairo_sha256.sha256_utils import (
    IV, compute_message_schedule, sha2_compress_function)

_sha256_input_chunk_size_felts = int(ids.SHA256_INPUT_CHUNK_SIZE_FELTS)
assert 0 <= _sha256_input_chunk_size_felts < 100

w = compute_message_schedule(memory.get_range(
    ids.sha256_start, _sha256_input_chunk_size_felts))
new_state = sha2_compress_function(IV, w)
segments.write_arg(ids.output, new_state)"#}),
(SHA256_MAIN_ARBITRARY_INPUT_LENGTH, indoc! {r#"from starkware.cairo.common.cairo_sha256.sha256_utils import (
    compute_message_schedule, sha2_compress_function)

_sha256_input_chunk_size_felts = int(ids.SHA256_INPUT_CHUNK_SIZE_FELTS)
assert 0 <= _sha256_input_chunk_size_felts < 100
_sha256_state_size_felts = int(ids.SHA256_STATE_SIZE_FELTS)
assert 0 <= _sha256_state_size_felts < 100
w = compute_message_schedule(memory.get_range(
    ids.sha256_start, _sha256_input_chunk_size_felts))
new_state = sha2_compress_function(memory.get_range(ids.state, _sha256_state_size_felts), w)
segments.write_arg(ids.output, new_state)"#}),
(SHA256_FINALIZE, indoc! {r#"# Add dummy pairs of input and output.
from starkware.cairo.common.cairo_sha256.sha256_utils import (
    IV, compute_message_schedule, sha2_compress_function)

_block_size = int(ids.BLOCK_SIZE)
assert 0 <= _block_size < 20
_sha256_input_chunk_size_felts = int(ids.SHA256_INPUT_CHUNK_SIZE_FELTS)
assert 0 <= _sha256_input_chunk_size_felts < 100

message = [0] * _sha256_input_chunk_size_felts
w = compute_message_schedule(message)
output = sha2_compress_function(IV, w)
padding = (message + IV + output) * (_block_size - 1)
segments.write_arg(ids.sha256_ptr_end, padding)"#}),
(KECCAK_WRITE_ARGS, indoc! {r#"segments.write_arg(ids.inputs, [ids.low % 2 ** 64, ids.low // 2 ** 64])
segments.write_arg(ids.inputs + 2, [ids.high % 2 ** 64, ids.high // 2 ** 64])"#}),
(COMPARE_BYTES_IN_WORD_NONDET, indoc! {r#"memory[ap] = to_felt_or_relocatable(ids.n_bytes < ids.BYTES_IN_WORD)"#}),
(COMPARE_KECCAK_FULL_RATE_IN_BYTES_NONDET, indoc! {r#"memory[ap] = to_felt_or_relocatable(ids.n_bytes >= ids.KECCAK_FULL_RATE_IN_BYTES)"#}),
(BLOCK_PERMUTATION, indoc! {r#"from starkware.cairo.common.keccak_utils.keccak_utils import keccak_func
_keccak_state_size_felts = int(ids.KECCAK_STATE_SIZE_FELTS)
assert 0 <= _keccak_state_size_felts < 100

output_values = keccak_func(memory.get_range(
    ids.keccak_ptr - _keccak_state_size_felts, _keccak_state_size_felts))
segments.write_arg(ids.keccak_ptr, output_values)"#}),
// The 0.10.3 whitelist uses this variant (instead of the one used by the common library), but both hints have the same behaviour
// We should check for future refactors that may discard one of the variants
(BLOCK_PERMUTATION_WHITELIST_V1, indoc! {r#"from starkware.cairo.common.cairo_keccak.keccak_utils import keccak_func
_keccak_state_size_felts = int(ids.KECCAK_STATE_SIZE_FELTS)
assert 0 <= _keccak_state_size_felts < 100

output_values = keccak_func(memory.get_range(
    ids.keccak_ptr - _keccak_state_size_felts, _keccak_state_size_felts))
segments.write_arg(ids.keccak_ptr, output_values)"#}),
(BLOCK_PERMUTATION_WHITELIST_V2, indoc! {r#"from starkware.cairo.common.cairo_keccak.keccak_utils import keccak_func
_keccak_state_size_felts = int(ids.KECCAK_STATE_SIZE_FELTS)
assert 0 <= _keccak_state_size_felts < 100
output_values = keccak_func(memory.get_range(
    ids.keccak_ptr_start, _keccak_state_size_felts))
segments.write_arg(ids.output, output_values)"#}),
(CAIRO_KECCAK_INPUT_IS_FULL_WORD, indoc! {r#"ids.full_word = int(ids.n_bytes >= 8)"#}),
(CAIRO_KECCAK_FINALIZE_V1, indoc! {r#"# Add dummy pairs of input and output.
_keccak_state_size_felts = int(ids.KECCAK_STATE_SIZE_FELTS)
_block_size = int(ids.BLOCK_SIZE)
assert 0 <= _keccak_state_size_felts < 100
assert 0 <= _block_size < 10
inp = [0] * _keccak_state_size_felts
padding = (inp + keccak_func(inp)) * _block_size
segments.write_arg(ids.keccak_ptr_end, padding)"#}),
(CAIRO_KECCAK_FINALIZE_V2, indoc! {r#"# Add dummy pairs of input and output.
_keccak_state_size_felts = int(ids.KECCAK_STATE_SIZE_FELTS)
_block_size = int(ids.BLOCK_SIZE)
assert 0 <= _keccak_state_size_felts < 100
assert 0 <= _block_size < 1000
inp = [0] * _keccak_state_size_felts
padding = (inp + keccak_func(inp)) * _block_size
segments.write_arg(ids.keccak_ptr_end, padding)"#}),
(FAST_EC_ADD_ASSIGN_NEW_X, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

slope = pack(ids.slope, PRIME)
x0 = pack(ids.point0.x, PRIME)
x1 = pack(ids.point1.x, PRIME)
y0 = pack(ids.point0.y, PRIME)

value = new_x = (pow(slope, 2, SECP_P) - x0 - x1) % SECP_P"#}),
(FAST_EC_ADD_ASSIGN_NEW_X_V2, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
SECP_P = 2**255-19

slope = pack(ids.slope, PRIME)
x0 = pack(ids.point0.x, PRIME)
x1 = pack(ids.point1.x, PRIME)
y0 = pack(ids.point0.y, PRIME)

value = new_x = (pow(slope, 2, SECP_P) - x0 - x1) % SECP_P"#}),
(FAST_EC_ADD_ASSIGN_NEW_X_V3, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

slope = pack(ids.slope, PRIME)
x0 = pack(ids.pt0.x, PRIME)
x1 = pack(ids.pt1.x, PRIME)
y0 = pack(ids.pt0.y, PRIME)

value = new_x = (pow(slope, 2, SECP_P) - x0 - x1) % SECP_P"#}),
(FAST_EC_ADD_ASSIGN_NEW_Y, indoc! {r#"value = new_y = (slope * (x0 - new_x) - y0) % SECP_P"#}),
(EC_MUL_INNER, indoc! {r#"memory[ap] = (ids.scalar % PRIME) % 2"#}),
(RELOCATE_SEGMENT, indoc! {r#"memory.add_relocation_rule(src_ptr=ids.src_ptr, dest_ptr=ids.dest_ptr)"#}),
(TEMPORARY_ARRAY, indoc! {r#"ids.temporary_array = segments.add_temp_segment()"#}),
(VERIFY_ECDSA_SIGNATURE, indoc! {r#"ecdsa_builtin.add_signature(ids.ecdsa_ptr.address_, (ids.signature_r, ids.signature_s))"#}),
(SPLIT_OUTPUT_0, indoc! {r#"ids.output0_low = ids.output0 & ((1 << 128) - 1)
ids.output0_high = ids.output0 >> 128"#}),
(SPLIT_OUTPUT_1, indoc! {r#"ids.output1_low = ids.output1 & ((1 << 128) - 1)
ids.output1_high = ids.output1 >> 128"#}),
(SPLIT_INPUT_3, indoc! {r#"ids.high3, ids.low3 = divmod(memory[ids.inputs + 3], 256)"#}),
(SPLIT_INPUT_6, indoc! {r#"ids.high6, ids.low6 = divmod(memory[ids.inputs + 6], 256 ** 2)"#}),
(SPLIT_INPUT_9, indoc! {r#"ids.high9, ids.low9 = divmod(memory[ids.inputs + 9], 256 ** 3)"#}),
(SPLIT_INPUT_12, indoc! {r#"ids.high12, ids.low12 = divmod(memory[ids.inputs + 12], 256 ** 4)"#}),
(SPLIT_INPUT_15, indoc! {r#"ids.high15, ids.low15 = divmod(memory[ids.inputs + 15], 256 ** 5)"#}),
(SPLIT_N_BYTES, indoc! {r#"ids.n_words_to_copy, ids.n_bytes_left = divmod(ids.n_bytes, ids.BYTES_IN_WORD)"#}),
(SPLIT_OUTPUT_MID_LOW_HIGH, indoc! {r#"tmp, ids.output1_low = divmod(ids.output1, 256 ** 7)
ids.output1_high, ids.output1_mid = divmod(tmp, 2 ** 128)"#}),
(NONDET_N_GREATER_THAN_10, indoc! {r#"memory[ap] = to_felt_or_relocatable(ids.n >= 10)"#}),
(NONDET_N_GREATER_THAN_2, indoc! {r#"memory[ap] = to_felt_or_relocatable(ids.n >= 2)"#}),
(RANDOM_EC_POINT, indoc! {r#"from starkware.crypto.signature.signature import ALPHA, BETA, FIELD_PRIME
from starkware.python.math_utils import random_ec_point
from starkware.python.utils import to_bytes

# Define a seed for random_ec_point that's dependent on all the input, so that:
#   (1) The added point s is deterministic.
#   (2) It's hard to choose inputs for which the builtin will fail.
seed = b"".join(map(to_bytes, [ids.p.x, ids.p.y, ids.m, ids.q.x, ids.q.y]))
ids.s.x, ids.s.y = random_ec_point(FIELD_PRIME, ALPHA, BETA, seed)"#}),
(CHAINED_EC_OP_RANDOM_EC_POINT, indoc! {r#"from starkware.crypto.signature.signature import ALPHA, BETA, FIELD_PRIME
from starkware.python.math_utils import random_ec_point
from starkware.python.utils import to_bytes

n_elms = ids.len
assert isinstance(n_elms, int) and n_elms >= 0, \
    f'Invalid value for len. Got: {n_elms}.'
if '__chained_ec_op_max_len' in globals():
    assert n_elms <= __chained_ec_op_max_len, \
        f'chained_ec_op() can only be used with len<={__chained_ec_op_max_len}. ' \
        f'Got: n_elms={n_elms}.'

# Define a seed for random_ec_point that's dependent on all the input, so that:
#   (1) The added point s is deterministic.
#   (2) It's hard to choose inputs for which the builtin will fail.
seed = b"".join(
    map(
        to_bytes,
        [
            ids.p.x,
            ids.p.y,
            *memory.get_range(ids.m, n_elms),
            *memory.get_range(ids.q.address_, 2 * n_elms),
        ],
    )
)
ids.s.x, ids.s.y = random_ec_point(FIELD_PRIME, ALPHA, BETA, seed)"#}),
(RECOVER_Y, indoc! {r#"from starkware.crypto.signature.signature import ALPHA, BETA, FIELD_PRIME
from starkware.python.math_utils import recover_y
ids.p.x = ids.x
# This raises an exception if `x` is not on the curve.
ids.p.y = recover_y(ids.x, ALPHA, BETA, FIELD_PRIME)"#}),
(PACK_MODN_DIV_MODN, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
from starkware.python.math_utils import div_mod, safe_div

N = 0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141
x = pack(ids.x, PRIME) % N
s = pack(ids.s, PRIME) % N
value = res = div_mod(x, s, N)"#}),
(XS_SAFE_DIV, indoc! {r#"value = k = safe_div(res * s - x, N)"#}),
// The following hints support the lib https://github.com/NethermindEth/research-basic-Cairo-operations-big-integers/blob/main/lib
(UINT384_UNSIGNED_DIV_REM, indoc! {r#"def split(num: int, num_bits_shift: int, length: int):
    a = []
    for _ in range(length):
        a.append( num & ((1 << num_bits_shift) - 1) )
        num = num >> num_bits_shift
    return tuple(a)

def pack(z, num_bits_shift: int) -> int:
    limbs = (z.d0, z.d1, z.d2)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

a = pack(ids.a, num_bits_shift = 128)
div = pack(ids.div, num_bits_shift = 128)
quotient, remainder = divmod(a, div)

quotient_split = split(quotient, num_bits_shift=128, length=3)
assert len(quotient_split) == 3

ids.quotient.d0 = quotient_split[0]
ids.quotient.d1 = quotient_split[1]
ids.quotient.d2 = quotient_split[2]

remainder_split = split(remainder, num_bits_shift=128, length=3)
ids.remainder.d0 = remainder_split[0]
ids.remainder.d1 = remainder_split[1]
ids.remainder.d2 = remainder_split[2]"#}),
(UINT384_SPLIT_128, indoc! {r#"ids.low = ids.a & ((1<<128) - 1)
ids.high = ids.a >> 128"#}),
(ADD_NO_UINT384_CHECK, indoc! {r#"sum_d0 = ids.a.d0 + ids.b.d0
ids.carry_d0 = 1 if sum_d0 >= ids.SHIFT else 0
sum_d1 = ids.a.d1 + ids.b.d1 + ids.carry_d0
ids.carry_d1 = 1 if sum_d1 >= ids.SHIFT else 0
sum_d2 = ids.a.d2 + ids.b.d2 + ids.carry_d1
ids.carry_d2 = 1 if sum_d2 >= ids.SHIFT else 0"#}),
(UINT384_SQRT, indoc! {r#"from starkware.python.math_utils import isqrt

def split(num: int, num_bits_shift: int, length: int):
    a = []
    for _ in range(length):
        a.append( num & ((1 << num_bits_shift) - 1) )
        num = num >> num_bits_shift
    return tuple(a)

def pack(z, num_bits_shift: int) -> int:
    limbs = (z.d0, z.d1, z.d2)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

a = pack(ids.a, num_bits_shift=128)
root = isqrt(a)
assert 0 <= root < 2 ** 192
root_split = split(root, num_bits_shift=128, length=3)
ids.root.d0 = root_split[0]
ids.root.d1 = root_split[1]
ids.root.d2 = root_split[2]"#}),
(SUB_REDUCED_A_AND_REDUCED_B, indoc! {r#"def split(num: int, num_bits_shift: int, length: int):
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

res = (a - b) % p


res_split = split(res, num_bits_shift=128, length=3)

ids.res.d0 = res_split[0]
ids.res.d1 = res_split[1]
ids.res.d2 = res_split[2]"#}),
(UNSIGNED_DIV_REM_UINT768_BY_UINT384, indoc! {r#"def split(num: int, num_bits_shift: int, length: int):
    a = []
    for _ in range(length):
        a.append( num & ((1 << num_bits_shift) - 1) )
        num = num >> num_bits_shift 
    return tuple(a)

def pack(z, num_bits_shift: int) -> int:
    limbs = (z.d0, z.d1, z.d2)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))
    
def pack_extended(z, num_bits_shift: int) -> int:
    limbs = (z.d0, z.d1, z.d2, z.d3, z.d4, z.d5)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

a = pack_extended(ids.a, num_bits_shift = 128)
div = pack(ids.div, num_bits_shift = 128)

quotient, remainder = divmod(a, div)

quotient_split = split(quotient, num_bits_shift=128, length=6)

ids.quotient.d0 = quotient_split[0]
ids.quotient.d1 = quotient_split[1]
ids.quotient.d2 = quotient_split[2]
ids.quotient.d3 = quotient_split[3]
ids.quotient.d4 = quotient_split[4]
ids.quotient.d5 = quotient_split[5]

remainder_split = split(remainder, num_bits_shift=128, length=3)
ids.remainder.d0 = remainder_split[0]
ids.remainder.d1 = remainder_split[1]
ids.remainder.d2 = remainder_split[2]"#}),
// equal to UNSIGNED_DIV_REM_UINT768_BY_UINT384 but with some whitespace removed
// in the `num = num >> num_bits_shift` and between `pack` and `pack_extended`
(UNSIGNED_DIV_REM_UINT768_BY_UINT384_STRIPPED, indoc! {r#"def split(num: int, num_bits_shift: int, length: int):
    a = []
    for _ in range(length):
        a.append( num & ((1 << num_bits_shift) - 1) )
        num = num >> num_bits_shift
    return tuple(a)

def pack(z, num_bits_shift: int) -> int:
    limbs = (z.d0, z.d1, z.d2)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

def pack_extended(z, num_bits_shift: int) -> int:
    limbs = (z.d0, z.d1, z.d2, z.d3, z.d4, z.d5)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

a = pack_extended(ids.a, num_bits_shift = 128)
div = pack(ids.div, num_bits_shift = 128)

quotient, remainder = divmod(a, div)

quotient_split = split(quotient, num_bits_shift=128, length=6)

ids.quotient.d0 = quotient_split[0]
ids.quotient.d1 = quotient_split[1]
ids.quotient.d2 = quotient_split[2]
ids.quotient.d3 = quotient_split[3]
ids.quotient.d4 = quotient_split[4]
ids.quotient.d5 = quotient_split[5]

remainder_split = split(remainder, num_bits_shift=128, length=3)
ids.remainder.d0 = remainder_split[0]
ids.remainder.d1 = remainder_split[1]
ids.remainder.d2 = remainder_split[2]"#}),
(UINT384_SIGNED_NN, indoc! {r#"memory[ap] = 1 if 0 <= (ids.a.d2 % PRIME) < 2 ** 127 else 0"#}),
(IMPORT_SECP256R1_ALPHA, indoc! {r#"from starkware.cairo.common.cairo_secp.secp256r1_utils import SECP256R1_ALPHA as ALPHA"#}),
(IMPORT_SECP256R1_N, indoc! {r#"from starkware.cairo.common.cairo_secp.secp256r1_utils import SECP256R1_N as N"#}),
(UINT384_GET_SQUARE_ROOT, indoc! {r#"from starkware.python.math_utils import is_quad_residue, sqrt

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
ids.sqrt_gx.d2 = split_root_gx[2]"#}),
(UINT256_GET_SQUARE_ROOT, indoc! {r#"from starkware.python.math_utils import is_quad_residue, sqrt

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
ids.sqrt_gx.high = split_root_gx[1]"#}),
(UINT384_DIV, indoc! {r#"from starkware.python.math_utils import div_mod

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
ids.b_inverse_mod_p.d2 = b_inverse_mod_p_split[2]"#}),
(INV_MOD_P_UINT256, indoc! {r#"from starkware.python.math_utils import div_mod

def split(a: int):
    return (a & ((1 << 128) - 1), a >> 128)

def pack(z, num_bits_shift: int) -> int:
    limbs = (z.low, z.high)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

a = pack(ids.a, 128)
b = pack(ids.b, 128)
p = pack(ids.p, 128)
# For python3.8 and above the modular inverse can be computed as follows:
# b_inverse_mod_p = pow(b, -1, p)
# Instead we use the python3.7-friendly function div_mod from starkware.python.math_utils
b_inverse_mod_p = div_mod(1, b, p)

b_inverse_mod_p_split = split(b_inverse_mod_p)

ids.b_inverse_mod_p.low = b_inverse_mod_p_split[0]
ids.b_inverse_mod_p.high = b_inverse_mod_p_split[1]"#}),
(HI_MAX_BITLEN, indoc! {r#"ids.len_hi = max(ids.scalar_u.d2.bit_length(), ids.scalar_v.d2.bit_length())-1"#}),
(QUAD_BIT, indoc! {r#"ids.quad_bit = (
    8 * ((ids.scalar_v >> ids.m) & 1)
    + 4 * ((ids.scalar_u >> ids.m) & 1)
    + 2 * ((ids.scalar_v >> (ids.m - 1)) & 1)
    + ((ids.scalar_u >> (ids.m - 1)) & 1)
)"#}),
(INV_MOD_P_UINT512, indoc! {r#"def pack_512(u, num_bits_shift: int) -> int:
    limbs = (u.d0, u.d1, u.d2, u.d3)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

x = pack_512(ids.x, num_bits_shift = 128)
p = ids.p.low + (ids.p.high << 128)
x_inverse_mod_p = pow(x,-1, p)

x_inverse_mod_p_split = (x_inverse_mod_p & ((1 << 128) - 1), x_inverse_mod_p >> 128)

ids.x_inverse_mod_p.low = x_inverse_mod_p_split[0]
ids.x_inverse_mod_p.high = x_inverse_mod_p_split[1]"#}),
(DI_BIT, indoc! {r#"ids.dibit = ((ids.scalar_u >> ids.m) & 1) + 2 * ((ids.scalar_v >> ids.m) & 1)"#}),
(EC_RECOVER_DIV_MOD_N_PACKED, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
from starkware.python.math_utils import div_mod, safe_div

N = pack(ids.n, PRIME)
x = pack(ids.x, PRIME) % N
s = pack(ids.s, PRIME) % N
value = res = div_mod(x, s, N)"#}),
(UINT512_UNSIGNED_DIV_REM, indoc! {r#"def split(num: int, num_bits_shift: int, length: int):
    a = []
    for _ in range(length):
        a.append( num & ((1 << num_bits_shift) - 1) )
        num = num >> num_bits_shift
    return tuple(a)

def pack(z, num_bits_shift: int) -> int:
    limbs = (z.low, z.high)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

def pack_extended(z, num_bits_shift: int) -> int:
    limbs = (z.d0, z.d1, z.d2, z.d3)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

x = pack_extended(ids.x, num_bits_shift = 128)
div = pack(ids.div, num_bits_shift = 128)

quotient, remainder = divmod(x, div)

quotient_split = split(quotient, num_bits_shift=128, length=4)

ids.quotient.d0 = quotient_split[0]
ids.quotient.d1 = quotient_split[1]
ids.quotient.d2 = quotient_split[2]
ids.quotient.d3 = quotient_split[3]

remainder_split = split(remainder, num_bits_shift=128, length=2)
ids.remainder.low = remainder_split[0]
ids.remainder.high = remainder_split[1]"#}),
(EC_RECOVER_SUB_A_B, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
from starkware.python.math_utils import div_mod, safe_div

a = pack(ids.a, PRIME)
b = pack(ids.b, PRIME)

value = res = a - b"#}),
(A_B_BITAND_1, indoc! {r#"ids.a_lsb = ids.a & 1
ids.b_lsb = ids.b & 1"#}),
(EC_RECOVER_PRODUCT_MOD, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
from starkware.python.math_utils import div_mod, safe_div

a = pack(ids.a, PRIME)
b = pack(ids.b, PRIME)
product = a * b
m = pack(ids.m, PRIME)

value = res = product % m"#}),
(UINT256_MUL_INV_MOD_P, indoc! {r#"from starkware.python.math_utils import div_mod

def split(a: int):
    return (a & ((1 << 128) - 1), a >> 128)

def pack(z, num_bits_shift: int) -> int:
    limbs = (z.low, z.high)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

a = pack(ids.a, 128)
b = pack(ids.b, 128)
p = pack(ids.p, 128)
# For python3.8 and above the modular inverse can be computed as follows:
# b_inverse_mod_p = pow(b, -1, p)
# Instead we use the python3.7-friendly function div_mod from starkware.python.math_utils
b_inverse_mod_p = div_mod(1, b, p)

b_inverse_mod_p_split = split(b_inverse_mod_p)

ids.b_inverse_mod_p.low = b_inverse_mod_p_split[0]
ids.b_inverse_mod_p.high = b_inverse_mod_p_split[1]"#}),
(EC_RECOVER_PRODUCT_DIV_M, indoc! {r#"value = k = product // m"#}),
(SQUARE_SLOPE_X_MOD_P, indoc! {r#"from starkware.cairo.common.cairo_secp.secp_utils import pack

slope = pack(ids.slope, PRIME)
x0 = pack(ids.point0.x, PRIME)
x1 = pack(ids.point1.x, PRIME)
y0 = pack(ids.point0.y, PRIME)

value = new_x = (pow(slope, 2, SECP_P) - x0 - x1) % SECP_P"#}),
(SPLIT_XX, indoc! {r#"PRIME = 2**255 - 19
II = pow(2, (PRIME - 1) // 4, PRIME)

xx = ids.xx.low + (ids.xx.high<<128)
x = pow(xx, (PRIME + 3) // 8, PRIME)
if (x * x - xx) % PRIME != 0:
    x = (x * II) % PRIME
if x % 2 != 0:
    x = PRIME - x
ids.x.low = x & ((1<<128)-1)
ids.x.high = x >> 128"#}),
(SKIP_NEXT_INSTRUCTION, indoc! {r#"skip_next_instruction()"#}, "test_utils"),
(PRINT_FELT, indoc! {r#"print(ids.x)"#}, "test_utils"),
(PRINT_ARR, indoc! {r#"print(bytes.fromhex(f"{ids.name:062x}").decode().replace('\x00',''))
arr = [memory[ids.arr + i] for i in range(ids.arr_len)]
print(arr)"#}, "test_utils"),
(PRINT_DICT, indoc! {r#"print(bytes.fromhex(f"{ids.name:062x}").decode().replace('\x00',''))
data = __dict_manager.get_dict(ids.dict_ptr)
print(
    {k: v if isinstance(v, int) else [memory[v + i] for i in range(ids.pointer_size)] for k, v in data.items()}
)"#}, "test_utils"),
(RUN_P_CIRCUIT, "from starkware.cairo.lang.builtins.modulo.mod_builtin_runner import ModBuiltinRunner\nassert builtin_runners[\"add_mod_builtin\"].instance_def.batch_size == 1\nassert builtin_runners[\"mul_mod_builtin\"].instance_def.batch_size == 1\n\nModBuiltinRunner.fill_memory(\n    memory=memory,\n    add_mod=(ids.add_mod_ptr.address_, builtin_runners[\"add_mod_builtin\"], ids.add_mod_n),\n    mul_mod=(ids.mul_mod_ptr.address_, builtin_runners[\"mul_mod_builtin\"], ids.mul_mod_n),\n)"),
(RUN_P_CIRCUIT_WITH_LARGE_BATCH_SIZE, "from starkware.cairo.lang.builtins.modulo.mod_builtin_runner import ModBuiltinRunner\nassert builtin_runners[\"add_mod_builtin\"].instance_def.batch_size == ids.BATCH_SIZE\nassert builtin_runners[\"mul_mod_builtin\"].instance_def.batch_size == ids.BATCH_SIZE\n\nModBuiltinRunner.fill_memory(\n    memory=memory,\n    add_mod=(ids.add_mod_ptr.address_, builtin_runners[\"add_mod_builtin\"], ids.add_mod_n),\n    mul_mod=(ids.mul_mod_ptr.address_, builtin_runners[\"mul_mod_builtin\"], ids.mul_mod_n),\n)"),
(NONDET_ELEMENTS_OVER_TEN, indoc! {r#"memory[ap] = to_felt_or_relocatable(ids.elements_end - ids.elements >= 10)"#}),
(NONDET_ELEMENTS_OVER_TWO, indoc! {r#"memory[ap] = to_felt_or_relocatable(ids.elements_end - ids.elements >= 2)"#}),
(EXCESS_BALANCE, indoc! {r#"from excess_balance import excess_balance_func

res = excess_balance_func(ids, memory, __dict_manager)

ids.check_account_value = res["account_value"]
ids.check_excess_balance = res["excess_balance"]
ids.check_margin_requirement_d = res["margin_requirement"]
ids.check_unrealized_pnl_d = res["unrealized_pnl"]"#})
}
