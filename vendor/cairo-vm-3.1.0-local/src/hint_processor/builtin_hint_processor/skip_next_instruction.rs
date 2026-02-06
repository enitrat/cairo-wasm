use crate::vm::errors::hint_errors::HintError;
use crate::vm::vm_core::VirtualMachine;

/// Prevent the execution of the next instruction
///
/// This hint doesn't belong to the Cairo common library
/// It's only added for testing purposes
pub fn skip_next_instruction(vm: &mut VirtualMachine) -> Result<(), HintError> {
    vm.skip_next_instruction_execution();
    Ok(())
}
