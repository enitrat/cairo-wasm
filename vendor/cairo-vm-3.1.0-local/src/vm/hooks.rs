//! VM hooks
//!
//! Make it possible to execute custom arbitrary code at different stages of the VM execution
//!
//! If added to the VM, hooks function will be called during the VM execution at specific stages.
//!
//! Available hooks:
//! - before_first_step, executed before entering the execution loop in [run_until_pc](CairoRunner::run_until_pc)
//! - pre_step_instruction, executed before each instruction_step in [step](VirtualMachine::step)
//! - post_step_instruction, executed after each instruction_step in [step](VirtualMachine::step)

use crate::stdlib::{any::Any, collections::HashMap, prelude::*, sync::Arc};

use crate::Felt252;

use super::{errors::vm_errors::VirtualMachineError, vm_core::VirtualMachine};
use crate::vm::runners::cairo_runner::CairoRunner;
use crate::{
    hint_processor::hint_processor_definition::HintProcessor, types::exec_scope::ExecutionScopes,
};

type BeforeFirstStepHookFunc = Arc<
    dyn Fn(&mut VirtualMachine, &[Box<dyn Any>]) -> Result<(), VirtualMachineError> + Sync + Send,
>;

type StepHookFunc = Arc<
    dyn Fn(
            &mut VirtualMachine,
            &mut dyn HintProcessor,
            &mut ExecutionScopes,
            &[Box<dyn Any>],
            &HashMap<String, Felt252>,
        ) -> Result<(), VirtualMachineError>
        + Sync
        + Send,
>;

/// The hooks to be executed during the VM run.
pub trait StepHooks {
    fn before_first_step(
        &mut self,
        vm: &mut VirtualMachine,
        hints_data: &[Box<dyn Any>],
    ) -> Result<(), VirtualMachineError>;

    fn pre_step_instruction(
        &mut self,
        vm: &mut VirtualMachine,
        hint_processor: &mut dyn HintProcessor,
        exec_scopes: &mut ExecutionScopes,
        hints_data: &[Box<dyn Any>],
        constants: &HashMap<String, Felt252>,
    ) -> Result<(), VirtualMachineError>;

    fn post_step_instruction(
        &mut self,
        vm: &mut VirtualMachine,
        hint_processor: &mut dyn HintProcessor,
        exec_scopes: &mut ExecutionScopes,
        hints_data: &[Box<dyn Any>],
        constants: &HashMap<String, Felt252>,
    ) -> Result<(), VirtualMachineError>;
}

/// The hooks to be executed during the VM run.
///
/// They can be individually ignored by setting them to [None].
#[derive(Clone, Default)]
pub struct Hooks {
    before_first_step: Option<BeforeFirstStepHookFunc>,
    pre_step_instruction: Option<StepHookFunc>,
    post_step_instruction: Option<StepHookFunc>,
}

impl Hooks {
    pub fn new(
        before_first_step: Option<BeforeFirstStepHookFunc>,
        pre_step_instruction: Option<StepHookFunc>,
        post_step_instruction: Option<StepHookFunc>,
    ) -> Self {
        Hooks {
            before_first_step,
            pre_step_instruction,
            post_step_instruction,
        }
    }
}

impl StepHooks for Hooks {
    fn before_first_step(
        &mut self,
        vm: &mut VirtualMachine,
        hint_data: &[Box<dyn Any>],
    ) -> Result<(), VirtualMachineError> {
        if let Some(before_first_step) = &self.before_first_step {
            return before_first_step(vm, hint_data);
        }
        Ok(())
    }

    fn pre_step_instruction(
        &mut self,
        vm: &mut VirtualMachine,
        hint_executor: &mut dyn HintProcessor,
        exec_scope: &mut ExecutionScopes,
        hints_data: &[Box<dyn Any>],
        constants: &HashMap<String, Felt252>,
    ) -> Result<(), VirtualMachineError> {
        if let Some(pre_step_instruction) = &self.pre_step_instruction {
            return pre_step_instruction(vm, hint_executor, exec_scope, hints_data, constants);
        }

        Ok(())
    }

    fn post_step_instruction(
        &mut self,
        vm: &mut VirtualMachine,
        hint_executor: &mut dyn HintProcessor,
        exec_scope: &mut ExecutionScopes,
        hints_data: &[Box<dyn Any>],
        constants: &HashMap<String, Felt252>,
    ) -> Result<(), VirtualMachineError> {
        if let Some(post_step_instruction) = &self.post_step_instruction {
            return post_step_instruction(vm, hint_executor, exec_scope, hints_data, constants);
        }

        Ok(())
    }
}

impl VirtualMachine {
    pub fn execute_before_first_step(
        &mut self,
        hint_data: &[Box<dyn Any>],
    ) -> Result<(), VirtualMachineError> {
        if let Some(mut hooks) = self.hooks.take() {
            let result = hooks.before_first_step(self, hint_data);
            self.hooks = Some(hooks);
            return result;
        }

        Ok(())
    }

    pub fn execute_pre_step_instruction(
        &mut self,
        hint_executor: &mut dyn HintProcessor,
        exec_scope: &mut ExecutionScopes,
        hint_data: &[Box<dyn Any>],
        constants: &HashMap<String, Felt252>,
    ) -> Result<(), VirtualMachineError> {
        if let Some(mut hooks) = self.hooks.take() {
            let result =
                hooks.pre_step_instruction(self, hint_executor, exec_scope, hint_data, constants);
            self.hooks = Some(hooks);
            return result;
        }

        Ok(())
    }

    pub fn execute_post_step_instruction(
        &mut self,
        hint_executor: &mut dyn HintProcessor,
        exec_scope: &mut ExecutionScopes,
        hint_data: &[Box<dyn Any>],
        constants: &HashMap<String, Felt252>,
    ) -> Result<(), VirtualMachineError> {
        if let Some(mut hooks) = self.hooks.take() {
            let result =
                hooks.post_step_instruction(self, hint_executor, exec_scope, hint_data, constants);
            self.hooks = Some(hooks);
            return result;
        }

        Ok(())
    }
}

impl CairoRunner {
    pub fn set_vm_hooks(&mut self, hooks: Box<dyn StepHooks>) {
        self.vm.hooks = Some(hooks);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
        types::program::Program, utils::test_utils::cairo_runner,
    };
    #[test]
    fn empty_hooks() {
        let program = Program::from_bytes(
            include_bytes!("../../../cairo_programs/sqrt.json"),
            Some("main"),
        )
        .expect("Call to `Program::from_file()` failed.");

        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        cairo_runner.vm.hooks = Some(Box::new(Hooks::new(None, None, None)));

        let end = cairo_runner.initialize(false).unwrap();
        assert!(cairo_runner.run_until_pc(end, &mut hint_processor).is_ok());
    }

    #[test]
    fn hook_failure() {
        let program = Program::from_bytes(
            include_bytes!("../../../cairo_programs/sqrt.json"),
            Some("main"),
        )
        .expect("Call to `Program::from_file()` failed.");

        fn before_first_step_hook(
            _vm: &mut VirtualMachine,
            _hint_data: &[Box<dyn Any>],
        ) -> Result<(), VirtualMachineError> {
            Err(VirtualMachineError::Unexpected)
        }

        fn pre_step_hook(
            _vm: &mut VirtualMachine,
            _hint_processor: &mut dyn HintProcessor,
            _exec_scope: &mut ExecutionScopes,
            _hint_data: &[Box<dyn Any>],
            _constants: &HashMap<String, Felt252>,
        ) -> Result<(), VirtualMachineError> {
            Err(VirtualMachineError::Unexpected)
        }

        fn post_step_hook(
            _vm: &mut VirtualMachine,
            _hint_processor: &mut dyn HintProcessor,
            _exec_scope: &mut ExecutionScopes,
            _hint_data: &[Box<dyn Any>],
            _constants: &HashMap<String, Felt252>,
        ) -> Result<(), VirtualMachineError> {
            Err(VirtualMachineError::Unexpected)
        }

        // Before first fail
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        cairo_runner.vm.hooks = Some(Box::new(Hooks::new(
            Some(Arc::new(before_first_step_hook)),
            None,
            None,
        )));

        let end = cairo_runner.initialize(false).unwrap();
        assert!(cairo_runner.run_until_pc(end, &mut hint_processor).is_err());

        // Pre step fail
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        cairo_runner.vm.hooks = Some(Box::new(Hooks::new(
            None,
            Some(Arc::new(pre_step_hook)),
            None,
        )));

        let end = cairo_runner.initialize(false).unwrap();
        assert!(cairo_runner.run_until_pc(end, &mut hint_processor).is_err());

        // Post step fail
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        cairo_runner.vm.hooks = Some(Box::new(Hooks::new(
            None,
            None,
            Some(Arc::new(post_step_hook)),
        )));

        let end = cairo_runner.initialize(false).unwrap();
        assert!(cairo_runner.run_until_pc(end, &mut hint_processor).is_err());
    }

    #[test]
    fn hook_success() {
        let program = Program::from_bytes(
            include_bytes!("../../../cairo_programs/sqrt.json"),
            Some("main"),
        )
        .expect("Call to `Program::from_file()` failed.");

        fn before_first_step_hook(
            _vm: &mut VirtualMachine,
            _hint_data: &[Box<dyn Any>],
        ) -> Result<(), VirtualMachineError> {
            Ok(())
        }

        fn pre_step_hook(
            _vm: &mut VirtualMachine,
            _hint_processor: &mut dyn HintProcessor,
            _exec_scope: &mut ExecutionScopes,
            _hint_data: &[Box<dyn Any>],
            _constants: &HashMap<String, Felt252>,
        ) -> Result<(), VirtualMachineError> {
            Ok(())
        }

        fn post_step_hook(
            _vm: &mut VirtualMachine,
            _hint_processor: &mut dyn HintProcessor,
            _exec_scope: &mut ExecutionScopes,
            _hint_data: &[Box<dyn Any>],
            _constants: &HashMap<String, Felt252>,
        ) -> Result<(), VirtualMachineError> {
            Ok(())
        }

        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        cairo_runner.vm.hooks = Some(Box::new(Hooks::new(
            Some(Arc::new(before_first_step_hook)),
            Some(Arc::new(pre_step_hook)),
            Some(Arc::new(post_step_hook)),
        )));

        let end = cairo_runner.initialize(false).unwrap();
        assert!(cairo_runner.run_until_pc(end, &mut hint_processor).is_ok());
    }
}
