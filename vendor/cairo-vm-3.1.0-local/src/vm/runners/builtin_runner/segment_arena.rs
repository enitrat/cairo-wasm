use crate::vm::errors::memory_errors::MemoryError;
use crate::{
    types::relocatable::{MaybeRelocatable, Relocatable},
    vm::vm_memory::memory_segments::MemorySegmentManager,
};

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use num_integer::div_ceil;

pub(crate) const ARENA_BUILTIN_SIZE: u32 = 3;
// The size of the builtin segment at the time of its creation.
const INITIAL_SEGMENT_SIZE: usize = ARENA_BUILTIN_SIZE as usize;

#[derive(Debug, Clone)]
pub struct SegmentArenaBuiltinRunner {
    base: Relocatable,
    pub(crate) included: bool,
    pub(crate) stop_ptr: Option<usize>,
}

impl SegmentArenaBuiltinRunner {
    pub(crate) fn new(included: bool) -> Self {
        SegmentArenaBuiltinRunner {
            base: Relocatable::from((0, 0)),
            included,
            stop_ptr: None,
        }
    }

    pub fn initialize_segments(&mut self, segments: &mut MemorySegmentManager) {
        let info = &[
            MaybeRelocatable::from(segments.add()),
            MaybeRelocatable::from(0),
            MaybeRelocatable::from(0),
        ];
        let segment_start = gen_arg(segments, info);
        // 0 + 3 can't fail
        self.base = (segment_start + INITIAL_SEGMENT_SIZE).unwrap();
    }

    pub fn get_used_cells(&self, segments: &MemorySegmentManager) -> Result<usize, MemoryError> {
        let used = segments
            .get_segment_used_size(self.base.segment_index as usize)
            .ok_or(MemoryError::MissingSegmentUsedSizes)?;
        if used < INITIAL_SEGMENT_SIZE {
            return Err(MemoryError::InvalidUsedSizeSegmentArena);
        }
        Ok(used - INITIAL_SEGMENT_SIZE)
    }

    pub fn initial_stack(&self) -> Vec<MaybeRelocatable> {
        if self.included {
            vec![MaybeRelocatable::from(self.base)]
        } else {
            vec![]
        }
    }

    pub fn get_used_instances(
        &self,
        segments: &MemorySegmentManager,
    ) -> Result<usize, MemoryError> {
        Ok(div_ceil(
            self.get_used_cells(segments)?,
            ARENA_BUILTIN_SIZE as usize,
        ))
    }

    pub fn base(&self) -> usize {
        self.base.segment_index as usize
    }
}

// Specific non-failling version of gen_arg used specifically for SegmentArenaBuiltinRunner
fn gen_arg(segments: &mut MemorySegmentManager, data: &[MaybeRelocatable; 3]) -> Relocatable {
    let base = segments.add();
    for (num, value) in data.iter().enumerate() {
        // 0 + 3 can't fail, inserting into newly created segment can't fail
        segments
            .memory
            .insert((base + num).unwrap(), value)
            .unwrap();
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::builtin_name::BuiltinName;
    use crate::vm::errors::runner_errors::RunnerError;
    use crate::{relocatable, utils::test_utils::*, vm::runners::builtin_runner::BuiltinRunner};
    #[cfg(not(feature = "std"))]
    use alloc::boxed::Box;
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn gen_arg_test() {
        let mut segments = MemorySegmentManager::new();
        let data = &[
            MaybeRelocatable::from(segments.add()),
            MaybeRelocatable::from(0),
            MaybeRelocatable::from(0),
        ];
        let base = gen_arg(&mut segments, data);
        assert_eq!(base, (1, 0).into());
        check_memory!(segments.memory, ((1, 0), (0, 0)), ((1, 1), 0), ((1, 2), 0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_instances() {
        let builtin = SegmentArenaBuiltinRunner::new(true);

        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![3]);

        assert_eq!(builtin.get_used_instances(&vm.segments), Ok(0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_instances_enum() {
        let builtin: BuiltinRunner = SegmentArenaBuiltinRunner::new(true).into();

        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![3]);

        assert_eq!(builtin.get_used_instances(&vm.segments), Ok(0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_stop_pointer() {
        let mut builtin: BuiltinRunner = SegmentArenaBuiltinRunner::new(true).into();

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![6]);

        let pointer = Relocatable::from((2, 2));
        // USED_CELLS = SEGMENT_USED_SIZE - INITIAL_SIZE = 6 - 3 = 3
        // NUM_INSTANCES = DIV_CEIL(USED_CELLS, CELLS_PER_INSTANCE) = DIV_CEIL(3, 3) = 1

        // STOP_PTR == BASE + NUM_INSTANCES *  CELLS_PER_INSTANCE
        // (0, 0) == (0, 3) + 1 * 3
        // (0, 0) == (0, 6)
        assert_eq!(
            builtin.final_stack(&vm.segments, pointer),
            Err(RunnerError::InvalidStopPointer(Box::new((
                BuiltinName::segment_arena,
                relocatable!(0, 6),
                relocatable!(0, 0)
            ))))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_valid() {
        let mut builtin: BuiltinRunner = SegmentArenaBuiltinRunner::new(false).into();

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (1, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer).unwrap(),
            Relocatable::from((2, 2))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_valid_from_enum() {
        let mut builtin: BuiltinRunner = SegmentArenaBuiltinRunner::new(false).into();

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (1, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer).unwrap(),
            Relocatable::from((2, 2))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_non_relocatable() {
        let mut builtin: BuiltinRunner = SegmentArenaBuiltinRunner::new(true).into();

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), 2)
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer),
            Err(RunnerError::NoStopPointer(Box::new(
                BuiltinName::segment_arena
            )))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_and_allocated_size_test() {
        let builtin: BuiltinRunner = SegmentArenaBuiltinRunner::new(true).into();

        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![3]);

        assert_eq!(
            builtin.get_used_cells_and_allocated_size(&vm),
            Ok((0_usize, 0))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_segments_for_output() {
        let mut builtin = SegmentArenaBuiltinRunner::new(true);
        let mut segments = MemorySegmentManager::new();
        builtin.initialize_segments(&mut segments);
        assert_eq!(builtin.base, (1, 3).into());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_initial_stack_for_output_with_base() {
        let mut builtin = SegmentArenaBuiltinRunner::new(true);
        builtin.base = relocatable!(1, 0);
        let initial_stack = builtin.initial_stack();
        assert_eq!(
            initial_stack[0].clone(),
            MaybeRelocatable::RelocatableValue((builtin.base() as isize, 0).into())
        );
        assert_eq!(initial_stack.len(), 1);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_missing_segment_used_sizes() {
        let builtin = BuiltinRunner::SegmentArena(SegmentArenaBuiltinRunner::new(true));
        let vm = vm!();

        assert_eq!(
            builtin.get_used_cells(&vm.segments),
            Err(MemoryError::MissingSegmentUsedSizes)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_empty() {
        let builtin = BuiltinRunner::SegmentArena(SegmentArenaBuiltinRunner::new(true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(
            builtin.get_used_cells(&vm.segments),
            Err(MemoryError::InvalidUsedSizeSegmentArena)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells() {
        let builtin = BuiltinRunner::SegmentArena(SegmentArenaBuiltinRunner::new(true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(1));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_get_used_instances_missing_segments() {
        let builtin = BuiltinRunner::SegmentArena(SegmentArenaBuiltinRunner::new(true));
        let memory_segment_manager = MemorySegmentManager::new();

        assert_eq!(
            builtin.get_used_instances(&memory_segment_manager),
            Err(MemoryError::MissingSegmentUsedSizes)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_get_used_instances_valid() {
        let builtin = BuiltinRunner::SegmentArena(SegmentArenaBuiltinRunner::new(true));
        let mut memory_segment_manager = MemorySegmentManager::new();
        memory_segment_manager.segment_used_sizes = Some(vec![6]);
        // (SIZE(6) - INITIAL_SIZE(3)) / CELLS_PER_INSTANCE(3)
        assert_eq!(builtin.get_used_instances(&memory_segment_manager), Ok(1));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_deduce_memory_cell_output_builtin() {
        let builtin = BuiltinRunner::SegmentArena(SegmentArenaBuiltinRunner::new(true));
        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), 2)
        ];

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.deduce_memory_cell(pointer, &vm.segments.memory),
            Ok(None)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initial_stackincluded_test() {
        let ec_op_builtin: BuiltinRunner = SegmentArenaBuiltinRunner::new(true).into();
        assert_eq!(ec_op_builtin.initial_stack(), vec![mayberelocatable!(0, 0)])
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initial_stack_notincluded_test() {
        let ec_op_builtin = SegmentArenaBuiltinRunner::new(false);
        assert_eq!(ec_op_builtin.initial_stack(), Vec::new())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_add_validation_rule_enum() {
        let builtin: BuiltinRunner = SegmentArenaBuiltinRunner::new(true).into();
        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), 2)
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);
        builtin.add_validation_rule(&mut vm.segments.memory);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn cells_per_instance_enum() {
        let builtin: BuiltinRunner = SegmentArenaBuiltinRunner::new(true).into();
        assert_eq!(builtin.cells_per_instance(), ARENA_BUILTIN_SIZE)
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn n_input_cells_enum() {
        let builtin: BuiltinRunner = SegmentArenaBuiltinRunner::new(true).into();
        assert_eq!(builtin.n_input_cells(), ARENA_BUILTIN_SIZE)
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn instances_per_component_enum() {
        let builtin: BuiltinRunner = SegmentArenaBuiltinRunner::new(true).into();
        assert_eq!(builtin.instances_per_component(), 1)
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_air_private_input() {
        let builtin: BuiltinRunner = SegmentArenaBuiltinRunner::new(true).into();

        let segments = segments![((0, 0), 0), ((0, 1), 1), ((0, 2), 2), ((0, 3), 3)];
        assert!(builtin.air_private_input(&segments).is_empty());
    }
}
