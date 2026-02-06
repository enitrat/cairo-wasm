use crate::{
    hint_processor::hint_processor_definition::HintReference,
    serde::deserialize_program::ApTracking,
    stdlib::collections::HashMap,
    types::{exec_scope::ExecutionScopes, relocatable::MaybeRelocatable},
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};
use core::str::FromStr;

use num_bigint::{BigInt, BigUint};
use rust_decimal::Decimal;
use starknet_types_core::felt::Felt as Felt252;

use crate::{
    math_utils::{isqrt, signed_felt},
    stdlib::prelude::{String, ToString, Vec},
    types::relocatable::Relocatable,
    vm::vm_memory::memory::Memory,
};
use lazy_static::lazy_static;

use super::{
    dict_manager::DictManager,
    hint_utils::{
        get_constant_from_var_name, get_integer_from_var_name, get_ptr_from_var_name,
        insert_value_from_var_name,
    },
};

// General helper functions

lazy_static! {
    static ref DECIMAL_ADJUSTMENT_POSITIVE: Decimal = Decimal::from_scientific("1e8").unwrap();
    static ref DECIMAL_ADJUSTMENT: Decimal = Decimal::from_scientific("1e-8").unwrap();
    static ref DECIMAL_ADJUSTMENT_HALVED: Decimal = Decimal::from_scientific("1e-4").unwrap();
}

fn felt_to_scaled_decimal(f: &Felt252) -> Option<Decimal> {
    Some(Decimal::from_str_radix(&signed_felt(*f).to_string(), 10).ok()? * *DECIMAL_ADJUSTMENT)
}

fn felt_to_trimmed_str(f: &Felt252) -> Option<String> {
    Some(
        core::str::from_utf8(&f.to_bytes_be())
            .ok()?
            .trim_start_matches('\0')
            .to_string(),
    )
}

// Internal Data types

#[derive(Debug, PartialEq, Eq, Hash)]
struct Position {
    market: String,
    amount: Decimal,
    cost: Decimal,
    cached_funding: Decimal,
}

#[derive(Debug, PartialEq)]
struct MarginParams {
    market: String,
    imf_base: Decimal,
    imf_factor: Decimal,
    mmf_factor: Decimal,
    imf_shift: Decimal,
}

impl Position {
    fn read_from_memory(memory: &Memory, read_ptr: Relocatable) -> Option<Self> {
        Some(Position {
            market: felt_to_trimmed_str(memory.get_integer(read_ptr).ok()?.as_ref())?,
            amount: felt_to_scaled_decimal(
                memory.get_integer((read_ptr + 1_u32).ok()?).ok()?.as_ref(),
            )?,
            cost: felt_to_scaled_decimal(
                memory.get_integer((read_ptr + 2_u32).ok()?).ok()?.as_ref(),
            )?,
            cached_funding: felt_to_scaled_decimal(
                memory.get_integer((read_ptr + 3_u32).ok()?).ok()?.as_ref(),
            )?,
        })
    }
}

impl MarginParams {
    fn read_from_memory(memory: &Memory, read_ptr: Relocatable) -> Option<Self> {
        Some(MarginParams {
            market: felt_to_trimmed_str(memory.get_integer(read_ptr).ok()?.as_ref())?,
            imf_base: felt_to_scaled_decimal(
                memory.get_integer((read_ptr + 4_u32).ok()?).ok()?.as_ref(),
            )?,
            imf_factor: felt_to_scaled_decimal(
                memory.get_integer((read_ptr + 5_u32).ok()?).ok()?.as_ref(),
            )?,
            mmf_factor: felt_to_scaled_decimal(
                memory.get_integer((read_ptr + 6_u32).ok()?).ok()?.as_ref(),
            )?,
            imf_shift: felt_to_scaled_decimal(
                memory.get_integer((read_ptr + 7_u32).ok()?).ok()?.as_ref(),
            )?,
        })
    }

    fn imf(&self, abs_value: Decimal) -> Option<Decimal> {
        let diff = abs_value
            .checked_sub(self.imf_shift)?
            .checked_mul(*DECIMAL_ADJUSTMENT_POSITIVE)?;
        let max = BigUint::from_str(&Decimal::ZERO.max(diff.trunc()).to_string()).ok()?;
        let part_sqrt = isqrt(&max).ok()?;
        let part_sqrt = Decimal::from_str(&part_sqrt.to_string())
            .ok()?
            .checked_mul(*DECIMAL_ADJUSTMENT_HALVED)?;
        Some(self.imf_base.max(self.imf_factor.checked_mul(part_sqrt)?))
    }

    fn mmf(&self, abs_value: Decimal) -> Option<Decimal> {
        self.mmf_factor.checked_mul(self.imf(abs_value)?)
    }
}

// Excess Balance helpers

fn dict_ref_from_var_name<'a>(
    var_name: &'a str,
    vm: &'a VirtualMachine,
    dict_manager: &'a DictManager,
    ids_data: &'a HashMap<String, HintReference>,
    ap_tracking: &'a ApTracking,
) -> Option<&'a HashMap<MaybeRelocatable, MaybeRelocatable>> {
    let prices_cache_ptr = get_ptr_from_var_name(var_name, vm, ids_data, ap_tracking).ok()?;
    Some(
        dict_manager
            .get_tracker(prices_cache_ptr)
            .ok()?
            .get_dictionary_ref(),
    )
}

fn prices_dict(
    vm: &VirtualMachine,
    dict_manager: &DictManager,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Option<HashMap<String, Decimal>> {
    // Fetch dictionary
    let prices =
        dict_ref_from_var_name("prices_cache_ptr", vm, dict_manager, ids_data, ap_tracking)?;

    // Apply data type conversions
    let apply_conversion =
        |k: &MaybeRelocatable, v: &MaybeRelocatable| -> Option<(String, Decimal)> {
            Some((
                felt_to_trimmed_str(k.get_int_ref()?)?,
                felt_to_scaled_decimal(v.get_int_ref()?)?,
            ))
        };

    prices
        .iter()
        .map(|(k, v)| apply_conversion(k, v))
        .collect::<Option<_>>()
}

fn indices_dict(
    vm: &VirtualMachine,
    dict_manager: &DictManager,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Option<HashMap<String, Decimal>> {
    // Fetch dictionary
    let indices =
        dict_ref_from_var_name("indices_cache_ptr", vm, dict_manager, ids_data, ap_tracking)?;

    // Apply data type conversions
    let apply_conversion =
        |k: &MaybeRelocatable, v: &MaybeRelocatable| -> Option<(String, Decimal)> {
            Some((
                felt_to_trimmed_str(k.get_int_ref()?)?,
                felt_to_scaled_decimal(v.get_int_ref()?)?,
            ))
        };

    indices
        .iter()
        .map(|(k, v)| apply_conversion(k, v))
        .collect::<Option<_>>()
}

fn perps_dict(
    vm: &VirtualMachine,
    dict_manager: &DictManager,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Option<HashMap<String, MarginParams>> {
    // Fetch dictionary
    let perps = dict_ref_from_var_name("perps_cache_ptr", vm, dict_manager, ids_data, ap_tracking)?;

    // Apply data type conversions
    let apply_conversion =
        |k: &MaybeRelocatable, v: &MaybeRelocatable| -> Option<(String, MarginParams)> {
            Some((
                felt_to_trimmed_str(k.get_int_ref()?)?,
                MarginParams::read_from_memory(&vm.segments.memory, v.get_relocatable()?)?,
            ))
        };

    perps
        .iter()
        .map(|(k, v)| apply_conversion(k, v))
        .collect::<Option<_>>()
}

fn fees_dict(
    vm: &VirtualMachine,
    dict_manager: &DictManager,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Option<HashMap<Felt252, Decimal>> {
    // Fetch dictionary
    let fees = dict_ref_from_var_name("fees_cache_ptr", vm, dict_manager, ids_data, ap_tracking)?;

    // Apply data type conversions
    let apply_conversion =
        |k: &MaybeRelocatable, v: &MaybeRelocatable| -> Option<(Felt252, Decimal)> {
            Some((k.get_int()?, felt_to_scaled_decimal(v.get_int_ref()?)?))
        };

    fees.iter()
        .map(|(k, v)| apply_conversion(k, v))
        .collect::<Option<_>>()
}

fn balances_list(
    vm: &VirtualMachine,
    dict_manager: &DictManager,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Option<Vec<Position>> {
    // Fetch dictionary
    let balances = dict_ref_from_var_name(
        "perps_balances_cache_ptr",
        vm,
        dict_manager,
        ids_data,
        ap_tracking,
    )?;

    // Apply data type conversions
    let apply_conversion = |_, v: &MaybeRelocatable| -> Option<Position> {
        Position::read_from_memory(&vm.segments.memory, v.get_relocatable()?)
    };

    balances
        .iter()
        .map(|(k, v)| apply_conversion(k, v))
        .collect::<Option<_>>()
}

pub fn excess_balance_hint(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    constants: &HashMap<String, Felt252>,
    exec_scopes: &ExecutionScopes,
) -> Result<(), HintError> {
    // Fetch constants & variables
    let margin_check_type =
        get_integer_from_var_name("margin_check_type", vm, ids_data, ap_tracking)?;
    let margin_check_initial = get_constant_from_var_name("MARGIN_CHECK_INITIAL", constants)?;
    let token_assets_value_d =
        get_integer_from_var_name("token_assets_value_d", vm, ids_data, ap_tracking)?;
    let account = get_integer_from_var_name("account", vm, ids_data, ap_tracking)?;
    // Fetch DictManager
    let dict_manager_rc = exec_scopes.get_dict_manager()?;
    let dict_manager = dict_manager_rc.borrow();
    // Fetch dictionaries
    let prices = prices_dict(vm, &dict_manager, ids_data, ap_tracking)
        .ok_or_else(|| HintError::ExcessBalanceFailedToFecthDict("prices".into()))?;
    let indices = indices_dict(vm, &dict_manager, ids_data, ap_tracking)
        .ok_or_else(|| HintError::ExcessBalanceFailedToFecthDict("indices".into()))?;
    let perps = perps_dict(vm, &dict_manager, ids_data, ap_tracking)
        .ok_or_else(|| HintError::ExcessBalanceFailedToFecthDict("perps".into()))?;
    let fees = fees_dict(vm, &dict_manager, ids_data, ap_tracking)
        .ok_or_else(|| HintError::ExcessBalanceFailedToFecthDict("fees".into()))?;
    let balances = balances_list(vm, &dict_manager, ids_data, ap_tracking)
        .ok_or_else(|| HintError::ExcessBalanceFailedToFecthDict("balances".into()))?;

    // Fetch settelement price
    let settlement_asset = String::from("USDC-USD");
    let settlement_price = prices
        .get(&settlement_asset)
        .ok_or_else(|| HintError::ExcessBalanceKeyError("prices".into()))?;

    let mut unrealized_pnl = Decimal::ZERO;
    let mut unrealized_funding_pnl = Decimal::ZERO;
    let mut abs_balance_value = Decimal::ZERO;
    let mut position_margin = Decimal::ZERO;

    for position in balances {
        if position.market == settlement_asset {
            continue;
        }

        let price = prices
            .get(&position.market)
            .ok_or_else(|| HintError::ExcessBalanceKeyError("prices".into()))?;
        let funding_index = indices
            .get(&position.market)
            .ok_or_else(|| HintError::ExcessBalanceKeyError("indices".into()))?;
        let position_value = position
            .amount
            .checked_mul(*price)
            .ok_or_else(|| HintError::ExcessBalanceCalculationFailed("position_value".into()))?;
        let position_value_abs = position_value.abs();

        abs_balance_value = abs_balance_value
            .checked_add(position_value_abs)
            .ok_or_else(|| HintError::ExcessBalanceCalculationFailed("abs_balance_value".into()))?;

        let market_perps = perps
            .get(&position.market)
            .ok_or_else(|| HintError::ExcessBalanceKeyError("perps".into()))?;
        let margin_fraction = if &margin_check_type == margin_check_initial {
            market_perps.imf(position_value_abs)
        } else {
            market_perps.mmf(position_value_abs)
        }
        .ok_or_else(|| HintError::ExcessBalanceCalculationFailed("margin_fraction".into()))?;
        // position_margin += margin_fraction * position_value_abs
        position_margin = margin_fraction
            .checked_mul(position_value_abs)
            .and_then(|mul| position_margin.checked_add(mul))
            .ok_or_else(|| HintError::ExcessBalanceCalculationFailed("position_margin".into()))?;
        // unrealized_pnl += position_value - position.cost * settlement_price
        let calc_unrealized_pnl = |unrealized_pnl: Decimal,
                                   position: &Position,
                                   settlement_price: Decimal|
         -> Option<Decimal> {
            unrealized_pnl.checked_add(
                position_value.checked_sub(position.cost.checked_mul(settlement_price)?)?,
            )
        };
        unrealized_pnl = calc_unrealized_pnl(unrealized_pnl, &position, *settlement_price)
            .ok_or_else(|| HintError::ExcessBalanceCalculationFailed("unrealized_pnl".into()))?;
        // unrealized_funding_pnl += (position.cached_funding - funding_index) * position.amount*settlement_price
        let calc_unrealized_funding_pnl = |unrealized_funding_pnl: Decimal,
                                           position: &Position,
                                           funding_index: Decimal,
                                           settlement_price: Decimal|
         -> Option<Decimal> {
            unrealized_funding_pnl.checked_add(
                position
                    .cached_funding
                    .checked_sub(funding_index)?
                    .checked_mul(position.amount)?
                    .checked_mul(settlement_price)?,
            )
        };
        unrealized_funding_pnl = calc_unrealized_funding_pnl(
            unrealized_funding_pnl,
            &position,
            *funding_index,
            *settlement_price,
        )
        .ok_or_else(|| {
            HintError::ExcessBalanceCalculationFailed("unrealized_funding_pnl".into())
        })?;
    }

    // Calculate final results
    let token_assets_value_d = felt_to_scaled_decimal(&token_assets_value_d)
        .ok_or_else(|| HintError::ExcessBalanceCalculationFailed("account_value".into()))?;
    let account_value = unrealized_pnl
        .checked_add(unrealized_funding_pnl)
        .and_then(|sum| sum.checked_add(token_assets_value_d))
        .ok_or_else(|| HintError::ExcessBalanceCalculationFailed("account_value".into()))?;
    let fee = fees
        .get(&account)
        .ok_or_else(|| HintError::ExcessBalanceKeyError("fees".into()))?;
    let fee_provision = abs_balance_value
        .checked_mul(*fee)
        .ok_or_else(|| HintError::ExcessBalanceCalculationFailed("fee_provision".into()))?;
    let margin_requirement = position_margin
        .checked_add(fee_provision)
        .ok_or_else(|| HintError::ExcessBalanceCalculationFailed("margin_requirements".into()))?;
    let excess_balance = account_value
        .checked_sub(margin_requirement)
        .ok_or_else(|| HintError::ExcessBalanceCalculationFailed("excess_balance".into()))?;

    // Convert final results to Felt
    let felt_from_decimal = |d: Decimal| -> Option<Felt252> {
        Some(Felt252::from(
            BigInt::from_str(
                &(d.checked_mul(*DECIMAL_ADJUSTMENT_POSITIVE)?)
                    .trunc()
                    .to_string(),
            )
            .ok()?,
        ))
    };

    let account_value = felt_from_decimal(account_value)
        .ok_or_else(|| HintError::ExcessBalanceCalculationFailed("account_value".into()))?;
    let excess_balance = felt_from_decimal(excess_balance)
        .ok_or_else(|| HintError::ExcessBalanceCalculationFailed("excess_balance".into()))?;
    let margin_requirement = felt_from_decimal(margin_requirement)
        .ok_or_else(|| HintError::ExcessBalanceCalculationFailed("margin_requirement_d".into()))?;
    let unrealized_pnl = felt_from_decimal(unrealized_pnl)
        .ok_or_else(|| HintError::ExcessBalanceCalculationFailed("unrealized_pnl_d".into()))?;

    // Write results into memory
    insert_value_from_var_name(
        "check_account_value",
        account_value,
        vm,
        ids_data,
        ap_tracking,
    )?;
    insert_value_from_var_name(
        "check_excess_balance",
        excess_balance,
        vm,
        ids_data,
        ap_tracking,
    )?;
    insert_value_from_var_name(
        "check_margin_requirement_d",
        margin_requirement,
        vm,
        ids_data,
        ap_tracking,
    )?;
    insert_value_from_var_name(
        "check_unrealized_pnl_d",
        unrealized_pnl,
        vm,
        ids_data,
        ap_tracking,
    )
}

#[cfg(test)]
mod tests {
    use crate::stdlib::{cell::RefCell, rc::Rc};
    use core::str::FromStr;

    use super::*;
    use crate::{felt_str, utils::test_utils::*};

    #[test]
    fn test_read_position() {
        let memory = memory![
            ((0, 0), ("5176525270854594879110454268496", 10)),
            ((0, 1), 1000000000),
            ((0, 2), 20000),
            ((0, 3), 0)
        ];
        let expected_position = Position {
            market: String::from("AVAX-USD-PERP"),
            amount: Decimal::from_str("10.00000000").unwrap(),
            cost: Decimal::from_str("0.00020000").unwrap(),
            cached_funding: Decimal::from_scientific("0e-8").unwrap(),
        };
        assert_eq!(
            expected_position,
            Position::read_from_memory(&memory, (0, 0).into()).unwrap()
        )
    }

    #[test]
    fn test_read_margin_params() {
        let memory = memory![
            ((0, 0), ("20527877651862571847371805264", 10)),
            ((0, 4), 5000000),
            ((0, 5), 20000),
            ((0, 6), 50000000),
            ((0, 7), 20000000000000)
        ];
        let expected_position = MarginParams {
            market: String::from("BTC-USD-PERP"),
            imf_base: Decimal::from_str("0.05000000").unwrap(),
            imf_factor: Decimal::from_str("0.00020000").unwrap(),
            mmf_factor: Decimal::from_str("0.50000000").unwrap(),
            imf_shift: Decimal::from_str("200000.00000000").unwrap(),
        };
        assert_eq!(
            expected_position,
            MarginParams::read_from_memory(&memory, (0, 0).into()).unwrap()
        )
    }

    #[test]
    fn test_imf() {
        let abs_value = Decimal::from_str("459000.0000000000000000").unwrap();
        let margin_params = MarginParams {
            market: String::from("BTC-USD-PERP"),
            imf_base: Decimal::from_str("0.05000000").unwrap(),
            imf_factor: Decimal::from_str("0.00020000").unwrap(),
            mmf_factor: Decimal::from_str("0.50000000").unwrap(),
            imf_shift: Decimal::from_str("200000.00000000").unwrap(),
        };
        let expected_res = Decimal::from_str("0.101784080000").unwrap();
        assert_eq!(expected_res, margin_params.imf(abs_value).unwrap());
    }

    #[test]
    fn run_excess_balance_hint_succesful_trade() {
        // TEST DATA

        // INPUT VALUES
        // ids.margin_check_type 1
        // ids.MARGIN_CHECK_INITIAL 1
        // ids.token_assets_value_d 1005149999998000
        // ids.account 200
        // DICTIONARIES
        // prices {6044027408028715819619898970704: 5100000000000, 25783120691025710696626475600: 5100000000000, 5176525270854594879110454268496: 5100000000000, 21456356293159021401772216912: 5100000000000, 20527877651862571847371805264: 5100000000000, 6148332971604923204: 100000000}
        // indices {6044027408028715819619898970704: 0, 25783120691025710696626475600: 0, 5176525270854594879110454268496: 0, 21456356293159021401772216912: 0, 20527877651862571847371805264: 0}
        // perps {6044027408028715819619898970704: RelocatableValue(segment_index=1, offset=3092), 25783120691025710696626475600: RelocatableValue(segment_index=1, offset=3467), 5176525270854594879110454268496: RelocatableValue(segment_index=1, offset=3842), 21456356293159021401772216912: RelocatableValue(segment_index=1, offset=4217), 20527877651862571847371805264: RelocatableValue(segment_index=1, offset=4592)}
        // fees {100: 10000, 200: 10000}
        // balances {6044027408028715819619898970704: RelocatableValue(segment_index=1, offset=6406), 25783120691025710696626475600: RelocatableValue(segment_index=1, offset=6625), 5176525270854594879110454268496: RelocatableValue(segment_index=1, offset=6844), 21456356293159021401772216912: RelocatableValue(segment_index=1, offset=7063), 20527877651862571847371805264: RelocatableValue(segment_index=1, offset=18230)}
        // MEMORY VALUES REFERENCED BY DICTIONARIES
        // 1:3092 6044027408028715819619898970704
        // 1:3096 5000000
        // 1:3097 20000
        // 1:3098 50000000
        // 1:3099 20000000000000
        // 1:3467 25783120691025710696626475600
        // 1:3471 5000000
        // 1:3472 20000
        // 1:3473 50000000
        // 1:3474 20000000000000
        // 1:3842 5176525270854594879110454268496
        // 1:3846 5000000
        // 1:3847 20000
        // 1:3848 50000000
        // 1:3849 20000000000000
        // 1:4217 21456356293159021401772216912
        // 1:4221 5000000
        // 1:4222 20000
        // 1:4223 50000000
        // 1:4224 20000000000000
        // 1:4592 20527877651862571847371805264
        // 1:4596 5000000
        // 1:4597 20000
        // 1:4598 50000000
        // 1:4599 20000000000000
        // 1:6406 6044027408028715819619898970704
        // 1:6407 1000000000
        // 1:6408 20000
        // 1:6409 0
        // 1:6406 6044027408028715819619898970704
        // 1:6407 1000000000
        // 1:6408 20000
        // 1:6409 0
        // 1:6625 25783120691025710696626475600
        // 1:6626 1000000000
        // 1:6627 20000
        // 1:6628 0
        // 1:6625 25783120691025710696626475600
        // 1:6626 1000000000
        // 1:6627 20000
        // 1:6628 0
        // 1:6844 5176525270854594879110454268496
        // 1:6845 1000000000
        // 1:6846 20000
        // 1:6847 0
        // 1:6844 5176525270854594879110454268496
        // 1:6845 1000000000
        // 1:6846 20000
        // 1:6847 0
        // 1:7063 21456356293159021401772216912
        // 1:7064 1000000000
        // 1:7065 20000
        // 1:7066 0
        // 1:7063 21456356293159021401772216912
        // 1:7064 1000000000
        // 1:7065 20000
        // 1:7066 0
        // 1:18582 20527877651862571847371805264
        // 1:18583 900000000
        // 1:18584 18000
        // 1:18585 0
        // 1:18582 20527877651862571847371805264
        // 1:18583 900000000
        // 1:18584 18000
        // 1:18585 0
        // EXPECTED RESULTS
        // ids.check_account_value 1255049999900000
        // ids.check_excess_balance 1227636643508000
        // ids.check_margin_requirement_d 27413356392000
        // ids.check_unrealized_pnl_d 249899999902000

        // SETUP
        let mut vm = vm!();
        // CONSTANTS
        let constants = HashMap::from([("MARGIN_CHECK_INITIAL".to_string(), Felt252::ONE)]);
        // IDS
        vm.segments = segments!(
            ((1, 0), 1),                // ids.margin_check_type
            ((1, 1), 1005149999998000), // ids.token_assets_value_d
            ((1, 2), 200),              // ids.account
            ((1, 3), (2, 0)),           // ids.prices_cache_ptr
            ((1, 4), (3, 0)),           // ids.indices_cache_ptr
            ((1, 5), (4, 0)),           // ids.perps_cache_ptr
            ((1, 6), (5, 0)),           // ids.fees_cache_ptr
            ((1, 7), (6, 0)),           // ids.perps_balances_cache_ptr
            //((1, 8), ids.check_account_value)
            //((1, 9), ids.check_excess_balance)
            //((1, 10), ids.check_margin_requirement_d)
            //((1, 11), ids.check_unrealized_pnl_d)
            // Memory values referenced by hints
            ((1, 3092), 6044027408028715819619898970704),
            ((1, 3096), 5000000),
            ((1, 3097), 20000),
            ((1, 3098), 50000000),
            ((1, 3099), 20000000000000),
            ((1, 3467), 25783120691025710696626475600),
            ((1, 3471), 5000000),
            ((1, 3472), 20000),
            ((1, 3473), 50000000),
            ((1, 3474), 20000000000000),
            ((1, 3842), 5176525270854594879110454268496),
            ((1, 3846), 5000000),
            ((1, 3847), 20000),
            ((1, 3848), 50000000),
            ((1, 3849), 20000000000000),
            ((1, 4217), 21456356293159021401772216912),
            ((1, 4221), 5000000),
            ((1, 4222), 20000),
            ((1, 4223), 50000000),
            ((1, 4224), 20000000000000),
            ((1, 4592), 20527877651862571847371805264),
            ((1, 4596), 5000000),
            ((1, 4597), 20000),
            ((1, 4598), 50000000),
            ((1, 4599), 20000000000000),
            ((1, 6406), 6044027408028715819619898970704),
            ((1, 6407), 1000000000),
            ((1, 6408), 20000),
            ((1, 6409), 0),
            ((1, 6406), 6044027408028715819619898970704),
            ((1, 6407), 1000000000),
            ((1, 6408), 20000),
            ((1, 6409), 0),
            ((1, 6625), 25783120691025710696626475600),
            ((1, 6626), 1000000000),
            ((1, 6627), 20000),
            ((1, 6628), 0),
            ((1, 6625), 25783120691025710696626475600),
            ((1, 6626), 1000000000),
            ((1, 6627), 20000),
            ((1, 6628), 0),
            ((1, 6844), 5176525270854594879110454268496),
            ((1, 6845), 1000000000),
            ((1, 6846), 20000),
            ((1, 6847), 0),
            ((1, 6844), 5176525270854594879110454268496),
            ((1, 6845), 1000000000),
            ((1, 6846), 20000),
            ((1, 6847), 0),
            ((1, 7063), 21456356293159021401772216912),
            ((1, 7064), 1000000000),
            ((1, 7065), 20000),
            ((1, 7066), 0),
            ((1, 7063), 21456356293159021401772216912),
            ((1, 7064), 1000000000),
            ((1, 7065), 20000),
            ((1, 7066), 0),
            ((1, 18582), 20527877651862571847371805264),
            ((1, 18583), 900000000),
            ((1, 18584), 18000),
            ((1, 18585), 0),
            ((1, 18582), 20527877651862571847371805264),
            ((1, 18583), 900000000),
            ((1, 18584), 18000),
            ((1, 18585), 0)
        );
        vm.run_context.set_fp(12);
        let ids = ids_data![
            "margin_check_type",
            "token_assets_value_d",
            "account",
            "prices_cache_ptr",
            "indices_cache_ptr",
            "perps_cache_ptr",
            "fees_cache_ptr",
            "perps_balances_cache_ptr",
            "check_account_value",
            "check_excess_balance",
            "check_margin_requirement_d",
            "check_unrealized_pnl_d"
        ];
        // DICTIONARIES
        let mut exec_scopes = ExecutionScopes::new();
        let mut dict_manager = DictManager::new();
        // ids.prices_cache_ptr = (2, 0)
        dict_manager
            .new_dict(
                &mut vm,
                HashMap::from([
                    (
                        felt_str!("6044027408028715819619898970704").into(),
                        felt_str!("5100000000000").into(),
                    ),
                    (
                        felt_str!("25783120691025710696626475600").into(),
                        felt_str!("5100000000000").into(),
                    ),
                    (
                        felt_str!("5176525270854594879110454268496").into(),
                        felt_str!("5100000000000").into(),
                    ),
                    (
                        felt_str!("21456356293159021401772216912").into(),
                        felt_str!("5100000000000").into(),
                    ),
                    (
                        felt_str!("20527877651862571847371805264").into(),
                        felt_str!("5100000000000").into(),
                    ),
                    (
                        felt_str!("6148332971604923204").into(),
                        felt_str!("100000000").into(),
                    ),
                ]),
            )
            .unwrap();
        // ids.indices_cache_ptr = (3, 0)
        dict_manager
            .new_dict(
                &mut vm,
                HashMap::from([
                    (
                        felt_str!("6044027408028715819619898970704").into(),
                        Felt252::ZERO.into(),
                    ),
                    (
                        felt_str!("25783120691025710696626475600").into(),
                        Felt252::ZERO.into(),
                    ),
                    (
                        felt_str!("5176525270854594879110454268496").into(),
                        Felt252::ZERO.into(),
                    ),
                    (
                        felt_str!("21456356293159021401772216912").into(),
                        Felt252::ZERO.into(),
                    ),
                    (
                        felt_str!("20527877651862571847371805264").into(),
                        Felt252::ZERO.into(),
                    ),
                ]),
            )
            .unwrap();
        // ids.perps_cache_ptr = (4, 0)
        dict_manager
            .new_dict(
                &mut vm,
                HashMap::from([
                    (
                        felt_str!("6044027408028715819619898970704").into(),
                        (1, 3092).into(),
                    ),
                    (
                        felt_str!("25783120691025710696626475600").into(),
                        (1, 3467).into(),
                    ),
                    (
                        felt_str!("5176525270854594879110454268496").into(),
                        (1, 3842).into(),
                    ),
                    (
                        felt_str!("21456356293159021401772216912").into(),
                        (1, 4217).into(),
                    ),
                    (
                        felt_str!("20527877651862571847371805264").into(),
                        (1, 4592).into(),
                    ),
                ]),
            )
            .unwrap();
        // ids.fees_cache_ptr = (5, 0)
        dict_manager
            .new_dict(
                &mut vm,
                HashMap::from([
                    (Felt252::from(100).into(), Felt252::from(10000).into()),
                    (Felt252::from(200).into(), Felt252::from(10000).into()),
                ]),
            )
            .unwrap();
        // ids.perps_balances_cache_ptr = (6, 0)
        dict_manager
            .new_dict(
                &mut vm,
                HashMap::from([
                    (
                        felt_str!("6044027408028715819619898970704").into(),
                        (1, 6406).into(),
                    ),
                    (
                        felt_str!("25783120691025710696626475600").into(),
                        (1, 6625).into(),
                    ),
                    (
                        felt_str!("5176525270854594879110454268496").into(),
                        (1, 6844).into(),
                    ),
                    (
                        felt_str!("21456356293159021401772216912").into(),
                        (1, 7063).into(),
                    ),
                    (
                        felt_str!("20527877651862571847371805264").into(),
                        (1, 18582).into(),
                    ),
                ]),
            )
            .unwrap();
        exec_scopes.insert_value("dict_manager", Rc::new(RefCell::new(dict_manager)));

        // EXECUTION
        assert!(excess_balance_hint(
            &mut vm,
            &ids,
            &ApTracking::default(),
            &constants,
            &exec_scopes
        )
        .is_ok());

        // CHECK MEMORY VALUES
        check_memory![
            vm.segments.memory,
            // ids.check_account_value
            ((1, 8), 1255049999900000),
            // ids.check_excess_balance
            ((1, 9), 1227636643508000),
            // ids.check_margin_requirement_d
            ((1, 10), 27413356392000),
            // ids.check_unrealized_pnl_d
            ((1, 11), 249899999902000)
        ];
    }

    #[test]
    fn run_excess_balance_hint_trade_failure() {
        // TEST DATA

        // INPUT VALUES
        // ids.margin_check_type 1
        // ids.MARGIN_CHECK_INITIAL 1
        // ids.token_assets_value_d 0
        // ids.account 100
        // DICTIONARIES
        // prices {6044027408028715819619898970704: 5100000000000, 25783120691025710696626475600: 5100000000000, 5176525270854594879110454268496: 5100000000000, 21456356293159021401772216912: 5100000000000, 20527877651862571847371805264: 5100000000000, 6148332971604923204: 100000000}
        // indices {6044027408028715819619898970704: 0, 25783120691025710696626475600: 0, 5176525270854594879110454268496: 0, 21456356293159021401772216912: 0, 20527877651862571847371805264: 0}
        // perps {6044027408028715819619898970704: RelocatableValue(segment_index=1, offset=3092), 25783120691025710696626475600: RelocatableValue(segment_index=1, offset=3467), 5176525270854594879110454268496: RelocatableValue(segment_index=1, offset=3842), 21456356293159021401772216912: RelocatableValue(segment_index=1, offset=4217), 20527877651862571847371805264: RelocatableValue(segment_index=1, offset=4592)}
        // fees {100: 10000, 200: 10000}
        // balances {6044027408028715819619898970704: RelocatableValue(segment_index=1, offset=6406), 25783120691025710696626475600: RelocatableValue(segment_index=1, offset=6625), 5176525270854594879110454268496: RelocatableValue(segment_index=1, offset=6844), 21456356293159021401772216912: RelocatableValue(segment_index=1, offset=7063), 20527877651862571847371805264: RelocatableValue(segment_index=1, offset=18230)}
        // MEMORY VALUES REFERENCED BY DICTIONARIES
        // 1:3092 6044027408028715819619898970704
        // 1:3096 5000000
        // 1:3097 20000
        // 1:3098 50000000
        // 1:3099 20000000000000
        // 1:3467 25783120691025710696626475600
        // 1:3471 5000000
        // 1:3472 20000
        // 1:3473 50000000
        // 1:3474 20000000000000
        // 1:3842 5176525270854594879110454268496
        // 1:3846 5000000
        // 1:3847 20000
        // 1:3848 50000000
        // 1:3849 20000000000000
        // 1:4217 21456356293159021401772216912
        // 1:4221 5000000
        // 1:4222 20000
        // 1:4223 50000000
        // 1:4224 20000000000000
        // 1:4592 20527877651862571847371805264
        // 1:4596 5000000
        // 1:4597 20000
        // 1:4598 50000000
        // 1:4599 20000000000000
        // 1:6406 6044027408028715819619898970704
        // 1:6407 0
        // 1:6408 0
        // 1:6409 0
        // 1:6406 6044027408028715819619898970704
        // 1:6407 0
        // 1:6408 0
        // 1:6409 0
        // 1:6625 25783120691025710696626475600
        // 1:6626 0
        // 1:6627 0
        // 1:6628 0
        // 1:6625 25783120691025710696626475600
        // 1:6626 0
        // 1:6627 0
        // 1:6628 0
        // 1:6844 5176525270854594879110454268496
        // 1:6845 0
        // 1:6846 0
        // 1:6847 0
        // 1:6844 5176525270854594879110454268496
        // 1:6845 0
        // 1:6846 0
        // 1:6847 0
        // 1:7063 21456356293159021401772216912
        // 1:7064 0
        // 1:7065 0
        // 1:7066 0
        // 1:7063 21456356293159021401772216912
        // 1:7064 0
        // 1:7065 0
        // 1:7066 0
        // 1:18230 20527877651862571847371805264
        // 1:18231 3618502788666131213697322783095070105623107215331596699973092056135772020481
        // 1:18232 3618502788666131213697322783095070105623107215331596699973092050985872020481
        // 1:18233 0
        // 1:18230 20527877651862571847371805264
        // 1:18231 3618502788666131213697322783095070105623107215331596699973092056135772020481
        // 1:18232 3618502788666131213697322783095070105623107215331596699973092050985872020481
        // 1:18233 0
        // EXPECTED RESULTS
        // ids.check_account_value 50000000000
        // ids.check_excess_balance 3618502788666131213697322783095070105623107215331596699973092055930362020481
        // ids.check_margin_requirement_d 255510000000
        // ids.check_unrealized_pnl_d 50000000000

        // SETUP
        let mut vm = vm!();
        // CONSTANTS
        let constants = HashMap::from([("MARGIN_CHECK_INITIAL".to_string(), Felt252::ONE)]);
        // IDS
        vm.segments = segments!(
            ((1, 0), 1),      // ids.margin_check_type
            ((1, 1), 0),      // ids.token_assets_value_d
            ((1, 2), 100),    // ids.account
            ((1, 3), (2, 0)), // ids.prices_cache_ptr
            ((1, 4), (3, 0)), // ids.indices_cache_ptr
            ((1, 5), (4, 0)), // ids.perps_cache_ptr
            ((1, 6), (5, 0)), // ids.fees_cache_ptr
            ((1, 7), (6, 0)), // ids.perps_balances_cache_ptr
            //((1, 8), ids.check_account_value)
            //((1, 9), ids.check_excess_balance)
            //((1, 10), ids.check_margin_requirement_d)
            //((1, 11), ids.check_unrealized_pnl_d)
            // Memory values referenced by hints
            ((1, 3092), 6044027408028715819619898970704),
            ((1, 3096), 5000000),
            ((1, 3097), 20000),
            ((1, 3098), 50000000),
            ((1, 3099), 20000000000000),
            ((1, 3467), 25783120691025710696626475600),
            ((1, 3471), 5000000),
            ((1, 3472), 20000),
            ((1, 3473), 50000000),
            ((1, 3474), 20000000000000),
            ((1, 3842), 5176525270854594879110454268496),
            ((1, 3846), 5000000),
            ((1, 3847), 20000),
            ((1, 3848), 50000000),
            ((1, 3849), 20000000000000),
            ((1, 4217), 21456356293159021401772216912),
            ((1, 4221), 5000000),
            ((1, 4222), 20000),
            ((1, 4223), 50000000),
            ((1, 4224), 20000000000000),
            ((1, 4592), 20527877651862571847371805264),
            ((1, 4596), 5000000),
            ((1, 4597), 20000),
            ((1, 4598), 50000000),
            ((1, 4599), 20000000000000),
            ((1, 6406), 6044027408028715819619898970704),
            ((1, 6407), 0),
            ((1, 6408), 0),
            ((1, 6409), 0),
            ((1, 6406), 6044027408028715819619898970704),
            ((1, 6407), 0),
            ((1, 6408), 0),
            ((1, 6409), 0),
            ((1, 6625), 25783120691025710696626475600),
            ((1, 6626), 0),
            ((1, 6627), 0),
            ((1, 6628), 0),
            ((1, 6625), 25783120691025710696626475600),
            ((1, 6626), 0),
            ((1, 6627), 0),
            ((1, 6628), 0),
            ((1, 6844), 5176525270854594879110454268496),
            ((1, 6845), 0),
            ((1, 6846), 0),
            ((1, 6847), 0),
            ((1, 6844), 5176525270854594879110454268496),
            ((1, 6845), 0),
            ((1, 6846), 0),
            ((1, 6847), 0),
            ((1, 7063), 21456356293159021401772216912),
            ((1, 7064), 0),
            ((1, 7065), 0),
            ((1, 7066), 0),
            ((1, 7063), 21456356293159021401772216912),
            ((1, 7064), 0),
            ((1, 7065), 0),
            ((1, 7066), 0),
            ((1, 18230), 20527877651862571847371805264),
            (
                (1, 18231),
                (
                    "3618502788666131213697322783095070105623107215331596699973092056135772020481",
                    10
                )
            ),
            (
                (1, 18232),
                (
                    "3618502788666131213697322783095070105623107215331596699973092050985872020481",
                    10
                )
            ),
            ((1, 18233), 0),
            ((1, 18230), 20527877651862571847371805264),
            (
                (1, 18231),
                (
                    "3618502788666131213697322783095070105623107215331596699973092056135772020481",
                    10
                )
            ),
            (
                (1, 18232),
                (
                    "3618502788666131213697322783095070105623107215331596699973092050985872020481",
                    10
                )
            ),
            ((1, 18233), 0),
        );
        vm.run_context.set_fp(12);
        let ids = ids_data![
            "margin_check_type",
            "token_assets_value_d",
            "account",
            "prices_cache_ptr",
            "indices_cache_ptr",
            "perps_cache_ptr",
            "fees_cache_ptr",
            "perps_balances_cache_ptr",
            "check_account_value",
            "check_excess_balance",
            "check_margin_requirement_d",
            "check_unrealized_pnl_d"
        ];
        // DICTIONARIES
        let mut exec_scopes = ExecutionScopes::new();
        let mut dict_manager = DictManager::new();
        // ids.prices_cache_ptr = (2, 0)
        dict_manager
            .new_dict(
                &mut vm,
                HashMap::from([
                    (
                        felt_str!("6044027408028715819619898970704").into(),
                        felt_str!("5100000000000").into(),
                    ),
                    (
                        felt_str!("25783120691025710696626475600").into(),
                        felt_str!("5100000000000").into(),
                    ),
                    (
                        felt_str!("5176525270854594879110454268496").into(),
                        felt_str!("5100000000000").into(),
                    ),
                    (
                        felt_str!("21456356293159021401772216912").into(),
                        felt_str!("5100000000000").into(),
                    ),
                    (
                        felt_str!("20527877651862571847371805264").into(),
                        felt_str!("5100000000000").into(),
                    ),
                    (
                        felt_str!("6148332971604923204").into(),
                        felt_str!("100000000").into(),
                    ),
                ]),
            )
            .unwrap();
        // ids.indices_cache_ptr = (3, 0)
        dict_manager
            .new_dict(
                &mut vm,
                HashMap::from([
                    (
                        felt_str!("6044027408028715819619898970704").into(),
                        Felt252::ZERO.into(),
                    ),
                    (
                        felt_str!("25783120691025710696626475600").into(),
                        Felt252::ZERO.into(),
                    ),
                    (
                        felt_str!("5176525270854594879110454268496").into(),
                        Felt252::ZERO.into(),
                    ),
                    (
                        felt_str!("21456356293159021401772216912").into(),
                        Felt252::ZERO.into(),
                    ),
                    (
                        felt_str!("20527877651862571847371805264").into(),
                        Felt252::ZERO.into(),
                    ),
                ]),
            )
            .unwrap();
        // ids.perps_cache_ptr = (4, 0)
        dict_manager
            .new_dict(
                &mut vm,
                HashMap::from([
                    (
                        felt_str!("6044027408028715819619898970704").into(),
                        (1, 3092).into(),
                    ),
                    (
                        felt_str!("25783120691025710696626475600").into(),
                        (1, 3467).into(),
                    ),
                    (
                        felt_str!("5176525270854594879110454268496").into(),
                        (1, 3842).into(),
                    ),
                    (
                        felt_str!("21456356293159021401772216912").into(),
                        (1, 4217).into(),
                    ),
                    (
                        felt_str!("20527877651862571847371805264").into(),
                        (1, 4592).into(),
                    ),
                ]),
            )
            .unwrap();
        // ids.fees_cache_ptr = (5, 0)
        dict_manager
            .new_dict(
                &mut vm,
                HashMap::from([
                    (Felt252::from(100).into(), Felt252::from(10000).into()),
                    (Felt252::from(200).into(), Felt252::from(10000).into()),
                ]),
            )
            .unwrap();
        // ids.perps_balances_cache_ptr = (6, 0)
        dict_manager
            .new_dict(
                &mut vm,
                HashMap::from([
                    (
                        felt_str!("6044027408028715819619898970704").into(),
                        (1, 6406).into(),
                    ),
                    (
                        felt_str!("25783120691025710696626475600").into(),
                        (1, 6625).into(),
                    ),
                    (
                        felt_str!("5176525270854594879110454268496").into(),
                        (1, 6844).into(),
                    ),
                    (
                        felt_str!("21456356293159021401772216912").into(),
                        (1, 7063).into(),
                    ),
                    (
                        felt_str!("20527877651862571847371805264").into(),
                        (1, 18230).into(),
                    ),
                ]),
            )
            .unwrap();
        exec_scopes.insert_value("dict_manager", Rc::new(RefCell::new(dict_manager)));

        // EXECUTION
        assert!(excess_balance_hint(
            &mut vm,
            &ids,
            &ApTracking::default(),
            &constants,
            &exec_scopes
        )
        .is_ok());

        // CHECK MEMORY VALUES
        check_memory![
            vm.segments.memory,
            // ids.check_account_value
            ((1, 8), 50000000000),
            // ids.check_excess_balance
            (
                (1, 9),
                (
                    "3618502788666131213697322783095070105623107215331596699973092055930362020481",
                    10
                )
            ),
            // ids.check_margin_requirement_d
            ((1, 10), 255510000000),
            // ids.check_unrealized_pnl_d
            ((1, 11), 50000000000)
        ];
    }
}
