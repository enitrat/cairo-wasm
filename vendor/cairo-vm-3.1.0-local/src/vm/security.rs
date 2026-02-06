use crate::stdlib::prelude::*;

use num_traits::ToPrimitive;

use super::{
    errors::{runner_errors::RunnerError, vm_errors::VirtualMachineError},
    runners::cairo_runner::{CairoRunner, RunnerMode},
};
use crate::types::relocatable::MaybeRelocatable;
use crate::Felt252;

/// Verify that the completed run in a runner is safe to be relocated and be
/// used by other Cairo programs.
///
/// Checks include:
///   - (Only if `verify_builtins` is set to true) All accesses to the builtin segments must be within the range defined by
///     the builtins themselves.
///   - There must not be accesses to the program segment outside the program
///     data range. This check will use the `program_segment_size` instead of the program data length if available.
///   - All addresses in memory must be real (not temporary)
///
/// Note: Each builtin is responsible for checking its own segments' data.
pub fn verify_secure_runner(
    runner: &CairoRunner,
    verify_builtins: bool,
    program_segment_size: Option<usize>,
) -> Result<(), VirtualMachineError> {
    let builtins_segment_info = match verify_builtins {
        true => runner.get_builtin_segments_info()?,
        false => Vec::new(),
    };
    // Check builtin segment out of bounds.
    for (index, stop_ptr) in builtins_segment_info {
        let current_size = runner
            .vm
            .segments
            .memory
            .data
            .get(index)
            .map(|segment| segment.len());
        // + 1 here accounts for maximum segment offset being segment.len() -1
        if current_size >= Some(stop_ptr + 1) {
            return Err(VirtualMachineError::OutOfBoundsBuiltinSegmentAccess);
        }
    }
    // Check out of bounds for program segment.
    let program_segment_index = runner
        .program_base
        .and_then(|rel| rel.segment_index.to_usize())
        .ok_or(RunnerError::NoProgBase)?;
    let program_segment_size =
        program_segment_size.unwrap_or(runner.program.shared_program_data.data.len());
    let program_length = runner
        .vm
        .segments
        .memory
        .data
        .get(program_segment_index)
        .map(|segment| segment.len());
    // + 1 here accounts for maximum segment offset being segment.len() -1
    if program_length >= Some(program_segment_size + 1) {
        return Err(VirtualMachineError::OutOfBoundsProgramSegmentAccess);
    }
    // Check that the addresses in memory are valid
    // This means that every temporary address has been properly relocated to a real address
    // Asumption: If temporary memory is empty, this means no temporary memory addresses were generated and all addresses in memory are real
    if !runner.vm.segments.memory.temp_data.is_empty() {
        for value in runner.vm.segments.memory.data.iter().flatten() {
            match value.get_value() {
                Some(MaybeRelocatable::RelocatableValue(addr)) if addr.segment_index < 0 => {
                    return Err(VirtualMachineError::InvalidMemoryValueTemporaryAddress(
                        Box::new(addr),
                    ))
                }
                _ => {}
            }
        }
    }
    for builtin in runner.vm.builtin_runners.iter() {
        builtin.run_security_checks(&runner.vm)?;
    }

    // Validate ret FP.
    let initial_fp = runner
        .get_initial_fp()
        .ok_or(VirtualMachineError::MissingInitialFp)?;
    let ret_fp_addr = (initial_fp - 2).map_err(VirtualMachineError::Math)?;
    let ret_fp = runner
        .vm
        .get_maybe(&ret_fp_addr)
        .ok_or(VirtualMachineError::MissingReturnFp(Box::new(ret_fp_addr)))?;
    let final_fp = runner.vm.get_fp();
    match ret_fp {
        MaybeRelocatable::RelocatableValue(value) => {
            if runner.runner_mode == RunnerMode::ProofModeCanonical && value != final_fp {
                return Err(VirtualMachineError::MismatchReturnFP(Box::new((
                    value, final_fp,
                ))));
            }
            if runner.runner_mode == RunnerMode::ExecutionMode && value.offset != final_fp.offset {
                return Err(VirtualMachineError::MismatchReturnFPOffset(Box::new((
                    value, final_fp,
                ))));
            }
        }
        MaybeRelocatable::Int(value) => {
            if Felt252::from(final_fp.offset) != value {
                return Err(VirtualMachineError::MismatchReturnFPFelt(Box::new((
                    value, final_fp,
                ))));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;

    use crate::types::builtin_name::BuiltinName;
    use crate::types::relocatable::Relocatable;

    use crate::Felt252;
    use crate::{relocatable, types::program::Program, utils::test_utils::*};
    use assert_matches::assert_matches;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_without_program_base() {
        let program = program!();

        let runner = cairo_runner!(program);

        assert_matches!(
            verify_secure_runner(&runner, true, None),
            Err(VirtualMachineError::RunnerError(RunnerError::NoProgBase))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_empty_memory() {
        let program = program!(main = Some(0),);
        let mut runner = cairo_runner!(program);
        runner.initialize(false).unwrap();
        // runner.vm.segments.compute_effective_sizes();
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        runner
            .end_run(false, false, &mut hint_processor, false)
            .unwrap();
        // At the end of the run, the ret_fp should be the base of the new ret_fp segment we added
        // to the stack at the start of the run.
        runner.vm.run_context.fp = 0;
        assert_matches!(verify_secure_runner(&runner, true, None), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_program_access_out_of_bounds() {
        let program = program!(main = Some(0),);
        let mut runner = cairo_runner!(program);

        runner.initialize(false).unwrap();

        runner.vm.segments = segments![((0, 0), 100)];
        runner.vm.segments.segment_used_sizes = Some(vec![1]);

        assert_matches!(
            verify_secure_runner(&runner, true, None),
            Err(VirtualMachineError::OutOfBoundsProgramSegmentAccess)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_program_with_program_size() {
        let program = program!(main = Some(0),);
        let mut runner = cairo_runner!(program);

        runner.initialize(false).unwrap();
        // We insert (1, 0) for ret_fp segment.
        runner.vm.segments = segments![((0, 0), 100), ((1, 0), 0)];
        runner.vm.segments.segment_used_sizes = Some(vec![1]);
        // At the end of the run, the ret_fp should be the base of the new ret_fp segment we added
        // to the stack at the start of the run.
        runner.vm.run_context.fp = 0;
        assert_matches!(verify_secure_runner(&runner, true, Some(1)), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_builtin_access_out_of_bounds() {
        let program = program!(main = Some(0), builtins = vec![BuiltinName::range_check],);
        let mut runner = cairo_runner!(program);

        runner.initialize(false).unwrap();
        runner.vm.builtin_runners[0].set_stop_ptr(0);
        runner.vm.segments.memory = memory![((2, 0), 1)];
        runner.vm.segments.segment_used_sizes = Some(vec![0, 0, 0, 0]);

        assert_matches!(
            verify_secure_runner(&runner, true, None),
            Err(VirtualMachineError::OutOfBoundsBuiltinSegmentAccess)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_builtin_access_correct() {
        let program = program!(main = Some(0), builtins = vec![BuiltinName::range_check],);
        let mut runner = cairo_runner!(program);

        runner.initialize(false).unwrap();
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        runner
            .end_run(false, false, &mut hint_processor, false)
            .unwrap();
        runner.vm.builtin_runners[0].set_stop_ptr(1);
        // Adding ((1, 1), (3, 0)) to the memory segment to simulate the ret_fp_segment.
        runner.vm.segments.memory = memory![((2, 0), 1), ((1, 1), (3, 0))];
        // At the end of the run, the ret_fp should be the base of the new ret_fp segment we added
        // to the stack at the start of the run.
        runner.vm.run_context.fp = 0;
        runner.vm.segments.segment_used_sizes = Some(vec![0, 0, 1, 0]);

        assert_matches!(verify_secure_runner(&runner, true, None), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_success() {
        let program = program!(
            data = vec![
                Felt252::ZERO.into(),
                Felt252::ZERO.into(),
                Felt252::ZERO.into(),
                Felt252::ZERO.into(),
            ],
            main = Some(0),
        );

        let mut runner = cairo_runner!(program);

        runner.initialize(false).unwrap();
        // We insert (1, 0) for ret_fp segment.
        runner.vm.segments.memory = memory![
            ((0, 0), (1, 0)),
            ((0, 1), (2, 1)),
            ((0, 2), (3, 2)),
            ((0, 3), (4, 3)),
            ((1, 0), 0)
        ];
        runner.vm.segments.segment_used_sizes = Some(vec![5, 1, 2, 3, 4]);
        // At the end of the run, the ret_fp should be the base of the new ret_fp segment we added
        // to the stack at the start of the run.
        runner.vm.run_context.fp = 0;

        assert_matches!(verify_secure_runner(&runner, true, None), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_temporary_memory_properly_relocated() {
        let program = program!(
            data = vec![
                Felt252::ZERO.into(),
                Felt252::ZERO.into(),
                Felt252::ZERO.into(),
                Felt252::ZERO.into(),
            ],
            main = Some(0),
        );

        let mut runner = cairo_runner!(program);

        // We insert (1, 0) for ret_fp segment.
        runner.initialize(false).unwrap();
        runner.vm.segments.memory = memory![
            ((0, 1), (1, 0)),
            ((0, 2), (2, 1)),
            ((0, 3), (3, 2)),
            ((-1, 0), (1, 2)),
            ((1, 0), 0)
        ];
        runner.vm.segments.segment_used_sizes = Some(vec![5, 1, 2, 3, 4]);
        // At the end of the run, the ret_fp should be the base of the new ret_fp segment we added
        // to the stack at the start of the run.
        runner.vm.run_context.fp = 0;

        assert_matches!(verify_secure_runner(&runner, true, None), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_temporary_memory_not_fully_relocated() {
        let program = program!(
            data = vec![
                Felt252::ZERO.into(),
                Felt252::ZERO.into(),
                Felt252::ZERO.into(),
                Felt252::ZERO.into(),
            ],
            main = Some(0),
        );

        let mut runner = cairo_runner!(program);

        runner.initialize(false).unwrap();
        // We insert (1, 0) for ret_fp segment.
        runner.vm.segments.memory = memory![
            ((0, 0), (1, 0)),
            ((0, 1), (2, 1)),
            ((0, 2), (-3, 2)),
            ((0, 3), (4, 3)),
            ((-1, 0), (1, 2)),
            ((1, 0), 0)
        ];
        runner.vm.segments.segment_used_sizes = Some(vec![5, 1, 2, 3, 4]);

        assert_matches!(
            verify_secure_runner(&runner, true, None),
            Err(VirtualMachineError::InvalidMemoryValueTemporaryAddress(
                bx
            )) if *bx == relocatable!(-3, 2)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_missing_initial_fp_error() {
        let program = program!(main = Some(0),);
        let mut runner = cairo_runner!(program);
        // init program base to avoid other errors.
        runner.program_base = Some(runner.vm.add_memory_segment());

        assert_matches!(
            verify_secure_runner(&runner, true, None),
            Err(VirtualMachineError::MissingInitialFp)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_ret_fp_address_not_in_memory() {
        let program = program!(main = Some(0),);
        let mut runner = cairo_runner!(program);
        runner.initialize(false).unwrap();
        // simulate empty memory.
        runner.vm.segments.memory = crate::vm::vm_memory::memory::Memory::new();
        assert_matches!(
            verify_secure_runner(&runner, true, None),
            Err(VirtualMachineError::MissingReturnFp(..))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_return_fp_not_equal_final_fp_proof_mode() {
        let program = program!(main = Some(0),);
        let mut runner = cairo_runner!(program);
        runner.initialize(false).unwrap();

        // Set the runner mode to ProofModeCanonical, so we expect
        // the return FP to be equal to final_fp.
        runner.runner_mode = RunnerMode::ProofModeCanonical;

        assert_matches!(
            verify_secure_runner(&runner, true, None),
            Err(VirtualMachineError::MismatchReturnFP(..))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_return_fp_offset_not_equal_final_fp_offset_execution_mode() {
        let program = program!(main = Some(0),);
        let mut runner = cairo_runner!(program);
        runner.initialize(false).unwrap();

        // ExecutionMode only requires offset equality, not the entire relocatable.
        assert_matches!(
            verify_secure_runner(&runner, true, None),
            Err(VirtualMachineError::MismatchReturnFPOffset(..))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_return_fp_felt_not_equal_final_fp_offse() {
        let program = program!(main = Some(0),);
        let mut runner = cairo_runner!(program);
        runner.initialize(false).unwrap();
        // Insert Felt(0) as the return FP.
        runner.vm.segments.memory = memory![((1, 0), 0)];

        // ExecutionMode only requires offset equality, not the entire relocatable.
        assert_matches!(
            verify_secure_runner(&runner, true, None),
            Err(VirtualMachineError::MismatchReturnFPFelt(..))
        );
    }
}
