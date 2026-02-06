use crate::hint_processor::hint_processor_definition::HintReference;
use crate::stdlib::collections::HashMap;
use crate::types::relocatable::Relocatable;
use crate::Felt252;
use crate::{
    serde::deserialize_program::ApTracking,
    types::exec_scope::ExecutionScopes,
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};
use num_traits::ToPrimitive;

use super::hint_utils::{get_integer_from_var_name, insert_value_from_var_name};

pub const GET_SIMULATED_BUILTIN_BASE: &str =
    "ids.new_ptr = get_simulated_builtin_base(ids.builtin_idx)";

/// Obtains the simulated builtin runner base, at the given index. The simulated
/// builtin runner must be initialized before the execution.
///
/// This hint is not defined in the original VM, and its declared for testing
/// purposes only.
pub fn get_simulated_builtin_base(
    vm: &mut VirtualMachine,
    _exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    _constants: &HashMap<String, Felt252>,
) -> Result<(), HintError> {
    // Obtain the simulated builtin runner from the builtin_idx variable.
    let builtin_idx = get_integer_from_var_name("builtin_idx", vm, ids_data, ap_tracking)?
        .to_usize()
        .ok_or(HintError::BigintToUsizeFail)?;
    let builtin_runner = &vm.simulated_builtin_runners[builtin_idx];

    // Set new_ptr with the value of the builtin runner base.
    insert_value_from_var_name(
        "new_ptr",
        Relocatable {
            segment_index: builtin_runner.base() as isize,
            offset: 0,
        },
        vm,
        ids_data,
        ap_tracking,
    )
}
