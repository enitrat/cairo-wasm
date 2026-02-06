use crate::air_private_input::{PrivateInput, PrivateInputPoseidonState};
use crate::stdlib::{cell::RefCell, collections::HashMap, prelude::*};
use crate::types::builtin_name::BuiltinName;
use crate::types::instance_definitions::poseidon_instance_def::{
    CELLS_PER_POSEIDON, INPUT_CELLS_PER_POSEIDON,
};
use crate::types::relocatable::{MaybeRelocatable, Relocatable};
use crate::vm::errors::memory_errors::MemoryError;
use crate::vm::errors::runner_errors::RunnerError;
use crate::vm::vm_memory::memory::Memory;
use crate::vm::vm_memory::memory_segments::MemorySegmentManager;
use crate::Felt252;
use num_integer::div_ceil;
use starknet_types_core::hash::Poseidon;

#[derive(Debug, Clone)]
pub struct PoseidonBuiltinRunner {
    pub base: usize,
    ratio: Option<u32>,
    pub(crate) stop_ptr: Option<usize>,
    pub(crate) included: bool,
    cache: RefCell<HashMap<Relocatable, Felt252>>,
}

impl PoseidonBuiltinRunner {
    pub fn new(ratio: Option<u32>, included: bool) -> Self {
        PoseidonBuiltinRunner {
            base: 0,
            ratio,
            stop_ptr: None,
            included,
            cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn initialize_segments(&mut self, segments: &mut MemorySegmentManager) {
        self.base = segments.add().segment_index as usize // segments.add() always returns a positive index
    }

    pub fn initial_stack(&self) -> Vec<MaybeRelocatable> {
        if self.included {
            vec![MaybeRelocatable::from((self.base as isize, 0))]
        } else {
            vec![]
        }
    }

    pub fn base(&self) -> usize {
        self.base
    }

    pub fn ratio(&self) -> Option<u32> {
        self.ratio
    }

    pub fn add_validation_rule(&self, _memory: &mut Memory) {}

    pub fn deduce_memory_cell(
        &self,
        address: Relocatable,
        memory: &Memory,
    ) -> Result<Option<MaybeRelocatable>, RunnerError> {
        let index = address.offset % CELLS_PER_POSEIDON as usize;
        if index < INPUT_CELLS_PER_POSEIDON as usize {
            return Ok(None);
        }
        if let Some(felt) = self.cache.borrow().get(&address) {
            return Ok(Some(felt.into()));
        }
        let first_input_addr = (address - index)?;
        let first_output_addr = (first_input_addr + INPUT_CELLS_PER_POSEIDON as usize)?;

        let mut input_felts = vec![];

        for i in 0..INPUT_CELLS_PER_POSEIDON as usize {
            let m_index = (first_input_addr + i)?;
            let val = match memory.get(&m_index) {
                Some(value) => *value
                    .get_int_ref()
                    .ok_or(RunnerError::BuiltinExpectedInteger(Box::new((
                        BuiltinName::poseidon,
                        m_index,
                    ))))?,
                _ => return Ok(None),
            };
            input_felts.push(val)
        }
        // n_input_cells is fixed to 3, so this try_into will never fail
        let mut poseidon_state: [Felt252; 3] = input_felts.try_into().unwrap();
        Poseidon::hades_permutation(&mut poseidon_state);
        for (i, elem) in poseidon_state.iter().enumerate() {
            self.cache
                .borrow_mut()
                .insert((first_output_addr + i)?, *elem);
        }

        Ok(self.cache.borrow().get(&address).map(|x| x.into()))
    }

    pub fn get_used_cells(&self, segments: &MemorySegmentManager) -> Result<usize, MemoryError> {
        segments
            .get_segment_used_size(self.base())
            .ok_or(MemoryError::MissingSegmentUsedSizes)
    }

    pub fn get_used_instances(
        &self,
        segments: &MemorySegmentManager,
    ) -> Result<usize, MemoryError> {
        let used_cells = self.get_used_cells(segments)?;
        Ok(div_ceil(used_cells, CELLS_PER_POSEIDON as usize))
    }

    pub fn air_private_input(&self, memory: &Memory) -> Vec<PrivateInput> {
        let mut private_inputs = vec![];
        if let Some(segment) = memory.data.get(self.base) {
            let segment_len = segment.len();
            for (index, off) in (0..segment_len)
                .step_by(CELLS_PER_POSEIDON as usize)
                .enumerate()
            {
                // Add the input cells of each poseidon instance to the private inputs
                if let (Ok(input_s0), Ok(input_s1), Ok(input_s2)) = (
                    memory.get_integer((self.base as isize, off).into()),
                    memory.get_integer((self.base as isize, off + 1).into()),
                    memory.get_integer((self.base as isize, off + 2).into()),
                ) {
                    private_inputs.push(PrivateInput::PoseidonState(PrivateInputPoseidonState {
                        index,
                        input_s0: *input_s0,
                        input_s1: *input_s1,
                        input_s2: *input_s2,
                    }))
                }
            }
        }
        private_inputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;
    use crate::relocatable;
    use crate::types::builtin_name::BuiltinName;
    use crate::types::program::Program;
    use crate::utils::test_utils::*;

    use crate::vm::runners::builtin_runner::BuiltinRunner;
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_instances() {
        let builtin = PoseidonBuiltinRunner::new(Some(10), true);

        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![1]);

        assert_eq!(builtin.get_used_instances(&vm.segments), Ok(1));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_instances_enum() {
        let builtin: BuiltinRunner = PoseidonBuiltinRunner::new(Some(10), true).into();

        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![1]);

        assert_eq!(builtin.get_used_instances(&vm.segments), Ok(1));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack() {
        let mut builtin: BuiltinRunner = PoseidonBuiltinRunner::new(Some(10), true).into();

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer).unwrap(),
            Relocatable::from((2, 1))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_stop_pointer() {
        let mut builtin: BuiltinRunner = PoseidonBuiltinRunner::new(Some(10), true).into();

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![999]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer),
            Err(RunnerError::InvalidStopPointer(Box::new((
                BuiltinName::poseidon,
                relocatable!(0, 1002),
                relocatable!(0, 0)
            ))))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_when_not_included() {
        let mut builtin: BuiltinRunner = PoseidonBuiltinRunner::new(Some(10), false).into();

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
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
        let mut builtin: BuiltinRunner = PoseidonBuiltinRunner::new(Some(10), true).into();

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
            Err(RunnerError::NoStopPointer(Box::new(BuiltinName::poseidon)))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_and_allocated_size_test() {
        let builtin: BuiltinRunner = PoseidonBuiltinRunner::new(Some(10), true).into();

        let program = program!(
            builtins = vec![BuiltinName::poseidon],
            data = vec_data!(
                (4612671182993129469_i64),
                (5189976364521848832_i64),
                (18446744073709551615_i128),
                (5199546496550207487_i64),
                (4612389712311386111_i64),
                (5198983563776393216_i64),
                (2),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (7),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(8),
        );

        let mut cairo_runner = cairo_runner!(program);
        cairo_runner.vm.segments.segment_used_sizes = Some(vec![0]);

        let mut hint_processor = BuiltinHintProcessor::new_empty();

        let address = cairo_runner.initialize(false).unwrap();

        cairo_runner
            .run_until_pc(address, &mut hint_processor)
            .unwrap();

        assert_eq!(
            builtin.get_used_cells_and_allocated_size(&cairo_runner.vm),
            Ok((0, 6))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units() {
        let builtin: BuiltinRunner = PoseidonBuiltinRunner::new(Some(10), true).into();

        let program = program!(
            builtins = vec![BuiltinName::poseidon],
            data = vec_data!(
                (4612671182993129469_i64),
                (5189976364521848832_i64),
                (18446744073709551615_i128),
                (5199546496550207487_i64),
                (4612389712311386111_i64),
                (5198983563776393216_i64),
                (2),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (7),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(8),
        );

        let mut cairo_runner = cairo_runner!(program);

        let mut hint_processor = BuiltinHintProcessor::new_empty();

        let address = cairo_runner.initialize(false).unwrap();

        cairo_runner
            .run_until_pc(address, &mut hint_processor)
            .unwrap();

        assert_eq!(builtin.get_allocated_memory_units(&cairo_runner.vm), Ok(6));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_missing_input_cells_ok() {
        let builtin = PoseidonBuiltinRunner::new(Some(10), false);

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        assert_eq!(
            builtin.deduce_memory_cell(relocatable!(0, 1), &vm.segments.memory),
            Ok(None)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_air_private_input() {
        let builtin: BuiltinRunner = PoseidonBuiltinRunner::new(None, true).into();

        let segments = segments![
            ((0, 0), 0),
            ((0, 1), 1),
            ((0, 2), 2),
            ((0, 3), 3),
            ((0, 4), 4),
            ((0, 5), 5),
            ((0, 6), 6),
            ((0, 7), 7),
            ((0, 8), 8),
            ((0, 9), 9),
            ((0, 10), 10),
            ((0, 11), 11)
        ];
        assert_eq!(
            builtin.air_private_input(&segments),
            (vec![
                PrivateInput::PoseidonState(PrivateInputPoseidonState {
                    index: 0,
                    input_s0: 0.into(),
                    input_s1: 1.into(),
                    input_s2: 2.into(),
                }),
                PrivateInput::PoseidonState(PrivateInputPoseidonState {
                    index: 1,
                    input_s0: 6.into(),
                    input_s1: 7.into(),
                    input_s2: 8.into(),
                }),
            ]),
        );
    }
}
