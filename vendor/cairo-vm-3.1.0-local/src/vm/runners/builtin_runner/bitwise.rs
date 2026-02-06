use crate::air_private_input::{PrivateInput, PrivateInputPair};
use crate::stdlib::{boxed::Box, vec::Vec};
use crate::Felt252;
use crate::{
    types::{
        instance_definitions::bitwise_instance_def::{CELLS_PER_BITWISE, TOTAL_N_BITS},
        relocatable::{MaybeRelocatable, Relocatable},
    },
    vm::{
        errors::{memory_errors::MemoryError, runner_errors::RunnerError},
        vm_memory::{memory::Memory, memory_segments::MemorySegmentManager},
    },
};
use num_integer::div_ceil;

#[derive(Debug, Clone)]
pub struct BitwiseBuiltinRunner {
    ratio: Option<u32>,
    pub base: usize,
    pub(crate) stop_ptr: Option<usize>,
    pub(crate) included: bool,
}

impl BitwiseBuiltinRunner {
    pub(crate) fn new(ratio: Option<u32>, included: bool) -> Self {
        BitwiseBuiltinRunner {
            base: 0,
            ratio,
            stop_ptr: None,
            included,
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

    pub fn deduce_memory_cell(
        &self,
        address: Relocatable,
        memory: &Memory,
    ) -> Result<Option<MaybeRelocatable>, RunnerError> {
        let index = address.offset % CELLS_PER_BITWISE as usize;
        if index <= 1 {
            return Ok(None);
        }
        let x_addr = (address - index)?;
        let y_addr = (x_addr + 1_usize)?;

        let (Ok(num_x), Ok(num_y)) = (memory.get_integer(x_addr), memory.get_integer(y_addr))
        else {
            return Ok(None);
        };

        // NOTE: we could operate on bytes here, but it caused a 20% slowdown
        // on several benchmarks.
        let to_limbs = |x_addr, x: &Felt252| -> Result<[u64; 4], RunnerError> {
            const LEADING_BITS: u64 = 0xf800000000000000;
            let limbs = x.to_le_digits();
            if limbs[3] & LEADING_BITS != 0 {
                return Err(RunnerError::IntegerBiggerThanPowerOfTwo(Box::new((
                    x_addr,
                    TOTAL_N_BITS,
                    *x,
                ))));
            }
            Ok(limbs)
        };
        let (limbs_x, limbs_y) = (to_limbs(x_addr, &num_x)?, to_limbs(y_addr, &num_y)?);
        let mut limbs_xy = [0u64; 4];
        for (xy, (x, y)) in limbs_xy
            .iter_mut()
            .zip(limbs_x.into_iter().zip(limbs_y.into_iter()))
        {
            *xy = match index {
                2 => x & y,
                3 => x ^ y,
                4 => x | y,
                _ => {
                    return Ok(None);
                }
            };
        }
        let mut bytes_xy = [0u8; 32];
        bytes_xy[..8].copy_from_slice(limbs_xy[0].to_le_bytes().as_slice());
        bytes_xy[8..16].copy_from_slice(limbs_xy[1].to_le_bytes().as_slice());
        bytes_xy[16..24].copy_from_slice(limbs_xy[2].to_le_bytes().as_slice());
        bytes_xy[24..].copy_from_slice(limbs_xy[3].to_le_bytes().as_slice());
        Ok(Some(MaybeRelocatable::from(Felt252::from_bytes_le_slice(
            &bytes_xy,
        ))))
    }

    pub fn get_used_cells(&self, segments: &MemorySegmentManager) -> Result<usize, MemoryError> {
        segments
            .get_segment_used_size(self.base)
            .ok_or(MemoryError::MissingSegmentUsedSizes)
    }

    pub fn get_used_diluted_check_units(&self, diluted_spacing: u32, diluted_n_bits: u32) -> usize {
        let total_n_bits = TOTAL_N_BITS;
        let mut partition = Vec::with_capacity(total_n_bits as usize);
        for i in (0..total_n_bits).step_by((diluted_spacing * diluted_n_bits) as usize) {
            for j in 0..diluted_spacing {
                if i + j < total_n_bits {
                    partition.push(i + j)
                };
            }
        }
        let partition_lengh = partition.len();
        let num_trimmed = partition
            .into_iter()
            .filter(|elem| elem + diluted_spacing * (diluted_n_bits - 1) + 1 > total_n_bits)
            .count();
        4 * partition_lengh + num_trimmed
    }

    pub fn get_used_instances(
        &self,
        segments: &MemorySegmentManager,
    ) -> Result<usize, MemoryError> {
        let used_cells = self.get_used_cells(segments)?;
        Ok(div_ceil(used_cells, CELLS_PER_BITWISE as usize))
    }

    pub fn air_private_input(&self, memory: &Memory) -> Vec<PrivateInput> {
        let mut private_inputs = vec![];
        if let Some(segment) = memory.data.get(self.base) {
            let segment_len = segment.len();
            for (index, off) in (0..segment_len)
                .step_by(CELLS_PER_BITWISE as usize)
                .enumerate()
            {
                // Add the input cells of each bitwise instance to the private inputs
                if let (Ok(x), Ok(y)) = (
                    memory.get_integer((self.base as isize, off).into()),
                    memory.get_integer((self.base as isize, off + 1).into()),
                ) {
                    private_inputs.push(PrivateInput::Pair(PrivateInputPair {
                        index,
                        x: *x,
                        y: *y,
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
    use crate::relocatable;
    use crate::types::builtin_name::BuiltinName;
    use crate::vm::errors::memory_errors::MemoryError;
    use crate::vm::runners::builtin_runner::BuiltinRunner;
    use crate::Felt252;
    use crate::{
        hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
        types::program::Program, utils::test_utils::*,
    };

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_instances() {
        let builtin = BitwiseBuiltinRunner::new(Some(10), true);
        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![1]);

        assert_eq!(builtin.get_used_instances(&vm.segments), Ok(1));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack() {
        let mut builtin: BuiltinRunner = BitwiseBuiltinRunner::new(Some(10), true).into();
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
        let mut builtin: BuiltinRunner = BitwiseBuiltinRunner::new(Some(10), true).into();
        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![995]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer),
            Err(RunnerError::InvalidStopPointer(Box::new((
                BuiltinName::bitwise,
                relocatable!(0, 995),
                relocatable!(0, 0)
            ))))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_when_notincluded() {
        let mut builtin: BuiltinRunner = BitwiseBuiltinRunner::new(Some(10), false).into();
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
        let mut builtin: BuiltinRunner = BitwiseBuiltinRunner::new(Some(10), true).into();
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
            Err(RunnerError::NoStopPointer(Box::new(BuiltinName::bitwise)))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_and_allocated_size_test() {
        let builtin: BuiltinRunner = BitwiseBuiltinRunner::new(Some(10), true).into();

        let program = program!(
            builtins = vec![BuiltinName::bitwise],
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
            Ok((0, 5))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units() {
        let builtin: BuiltinRunner = BitwiseBuiltinRunner::new(Some(10), true).into();

        let program = program!(
            builtins = vec![BuiltinName::pedersen, BuiltinName::bitwise],
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

        assert_eq!(builtin.get_allocated_memory_units(&cairo_runner.vm), Ok(5));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_bitwise_for_preset_memory_valid_and() {
        let memory = memory![((0, 5), 10), ((0, 6), 12), ((0, 7), 0)];
        let builtin = BitwiseBuiltinRunner::new(Some(256), true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 7)), &memory);
        assert_eq!(result, Ok(Some(MaybeRelocatable::from(Felt252::from(8)))));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_bitwise_for_preset_memory_valid_xor() {
        let memory = memory![((0, 5), 10), ((0, 6), 12), ((0, 8), 0)];
        let builtin = BitwiseBuiltinRunner::new(Some(256), true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 8)), &memory);
        assert_eq!(result, Ok(Some(MaybeRelocatable::from(Felt252::from(6)))));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_bitwise_for_preset_memory_valid_or() {
        let memory = memory![((0, 5), 10), ((0, 6), 12), ((0, 9), 0)];
        let builtin = BitwiseBuiltinRunner::new(Some(256), true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 9)), &memory);
        assert_eq!(result, Ok(Some(MaybeRelocatable::from(Felt252::from(14)))));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_bitwise_for_preset_memory_incorrect_offset() {
        let memory = memory![((0, 3), 10), ((0, 4), 12), ((0, 5), 0)];
        let builtin = BitwiseBuiltinRunner::new(Some(256), true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 5)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_bitwise_for_preset_memory_no_values_to_operate() {
        let memory = memory![((0, 5), 12), ((0, 7), 0)];
        let builtin = BitwiseBuiltinRunner::new(Some(256), true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 5)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_missing_segment_used_sizes() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(Some(256), true));
        let vm = vm!();

        assert_eq!(
            builtin.get_used_cells(&vm.segments),
            Err(MemoryError::MissingSegmentUsedSizes)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_empty() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(Some(256), true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(Some(256), true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(4));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_diluted_check_units_a() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(Some(256), true));
        assert_eq!(builtin.get_used_diluted_check_units(12, 2), 535);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_diluted_check_units_b() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(Some(256), true));
        assert_eq!(builtin.get_used_diluted_check_units(30, 56), 150);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_diluted_check_units_c() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(Some(256), true));
        assert_eq!(builtin.get_used_diluted_check_units(50, 25), 250);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_air_private_input() {
        let builtin: BuiltinRunner = BitwiseBuiltinRunner::new(Some(256), true).into();

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
            ((0, 11), 11),
            ((0, 12), 12),
            ((0, 13), 13),
            ((0, 14), 14)
        ];
        assert_eq!(
            builtin.air_private_input(&segments),
            (vec![
                PrivateInput::Pair(PrivateInputPair {
                    index: 0,
                    x: 0.into(),
                    y: 1.into()
                }),
                PrivateInput::Pair(PrivateInputPair {
                    index: 1,
                    x: 5.into(),
                    y: 6.into()
                }),
                PrivateInput::Pair(PrivateInputPair {
                    index: 2,
                    x: 10.into(),
                    y: 11.into()
                }),
            ]),
        );
    }
}
