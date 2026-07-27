//! Stdout capture for browser runs, implemented without patching `cairo-lang-runner`.
//!
//! `println!` in Cairo lowers to a [`CoreHint::DebugPrint`] hint, which the upstream hint processor
//! executes by writing to the process stdout. In a browser there is no process stdout, so the
//! output would be lost.
//!
//! Rather than teaching the upstream hint processor to buffer its output, this wraps it: the
//! wrapper intercepts `DebugPrint` and appends to its own buffer, and delegates every other hint
//! untouched. `SierraCasmRunner::prepare_starknet_context` hands out the inner processor precisely
//! so it can be wrapped this way, which keeps this crate a pure addition to the workspace.
//!
//! Known limitation: output produced *inside* a simulated Starknet contract call is not captured.
//! Nested calls run on their own `SierraCasmRunner` internally, which this wrapper never sees.
//! Capturing those would require upstream's `RunResultStarknet` to carry the nested output.

use std::any::Any;
use std::sync::Arc;

use cairo_lang_casm::hints::{CoreHint, CoreHintBase, Hint};
use cairo_lang_runner::casm_run::{
    CairoHintProcessor, StarknetHintProcessor, format_for_debug, read_felts,
};
use cairo_lang_runner::{StarknetExecutionResources, StarknetState};
use cairo_vm::hint_processor::hint_processor_definition::{HintProcessorLogic, HintReference};
use cairo_vm::serde::deserialize_program::ApTracking;
use cairo_vm::types::exec_scope::ExecutionScopes;
use cairo_vm::vm::errors::hint_errors::HintError;
use cairo_vm::vm::errors::vm_errors::VirtualMachineError;
use cairo_vm::vm::runners::cairo_runner::{ResourceTracker, RunResources};
use cairo_vm::vm::vm_core::VirtualMachine;
use starknet_types_core::felt::Felt as Felt252;

/// Wraps a [`CairoHintProcessor`], capturing `DebugPrint` output into an in-memory buffer.
pub struct StdoutCapturingHintProcessor<'a> {
    inner: CairoHintProcessor<'a>,
    stdout: String,
}

impl<'a> StdoutCapturingHintProcessor<'a> {
    pub fn new(inner: CairoHintProcessor<'a>) -> Self {
        Self { inner, stdout: String::new() }
    }

    /// Returns the output captured so far, leaving the buffer empty.
    pub fn take_stdout(&mut self) -> String {
        std::mem::take(&mut self.stdout)
    }
}

impl HintProcessorLogic for StdoutCapturingHintProcessor<'_> {
    fn execute_hint(
        &mut self,
        vm: &mut VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
        hint_data: &Box<dyn Any>,
    ) -> Result<(), HintError> {
        // Only `DebugPrint` is handled here; everything else keeps upstream's behavior.
        if let Some(Hint::Core(CoreHintBase::Core(CoreHint::DebugPrint { start, end }))) =
            hint_data.downcast_ref::<Hint>()
        {
            let felts = read_felts(vm, start, end)?;
            self.stdout.push_str(&format_for_debug(felts.into_iter()));
            return Ok(());
        }
        self.inner.execute_hint(vm, exec_scopes, hint_data)
    }

    #[expect(clippy::disallowed_types)]
    fn compile_hint(
        &self,
        hint_code: &str,
        ap_tracking_data: &ApTracking,
        reference_ids: &std::collections::HashMap<String, usize>,
        references: &[HintReference],
        accessible_scopes: &[String],
        constants: Arc<std::collections::HashMap<String, Felt252>>,
    ) -> Result<Box<dyn Any>, VirtualMachineError> {
        self.inner.compile_hint(
            hint_code,
            ap_tracking_data,
            reference_ids,
            references,
            accessible_scopes,
            constants,
        )
    }
}

impl ResourceTracker for StdoutCapturingHintProcessor<'_> {
    fn consumed(&self) -> bool {
        self.inner.consumed()
    }

    fn consume_step(&mut self) {
        self.inner.consume_step()
    }

    fn get_n_steps(&self) -> Option<usize> {
        self.inner.get_n_steps()
    }

    fn run_resources(&self) -> &RunResources {
        self.inner.run_resources()
    }
}

impl StarknetHintProcessor for StdoutCapturingHintProcessor<'_> {
    fn take_starknet_state(&mut self) -> StarknetState {
        self.inner.take_starknet_state()
    }

    fn take_syscalls_used_resources(&mut self) -> StarknetExecutionResources {
        self.inner.take_syscalls_used_resources()
    }
}
