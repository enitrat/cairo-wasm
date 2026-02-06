use crate::air_private_input::{PrivateInput, PrivateInputKeccakState};
use crate::math_utils::safe_div_usize;
use crate::stdlib::{cell::RefCell, collections::HashMap, prelude::*};
use crate::types::builtin_name::BuiltinName;
use crate::types::instance_definitions::keccak_instance_def::{
    CELLS_PER_KECCAK, INPUT_CELLS_PER_KECCAK,
};
use crate::types::relocatable::{MaybeRelocatable, Relocatable};
use crate::vm::errors::memory_errors::MemoryError;
use crate::vm::errors::runner_errors::RunnerError;
use crate::vm::vm_memory::memory::Memory;
use crate::vm::vm_memory::memory_segments::MemorySegmentManager;
use crate::Felt252;
use lazy_static::lazy_static;
use num_bigint::BigUint;
use num_integer::div_ceil;

const KECCAK_FELT_BYTE_SIZE: usize = 25; // 200 / 8
const BITS: u32 = 200;
lazy_static! {
    static ref KECCAK_INPUT_MAX: Felt252 = Felt252::TWO.pow(BITS);
}

#[derive(Debug, Clone)]
pub struct KeccakBuiltinRunner {
    ratio: Option<u32>,
    pub base: usize,
    pub(crate) stop_ptr: Option<usize>,
    pub(crate) included: bool,
    cache: RefCell<HashMap<Relocatable, Felt252>>,
}

impl KeccakBuiltinRunner {
    pub fn new(ratio: Option<u32>, included: bool) -> Self {
        KeccakBuiltinRunner {
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

    pub fn deduce_memory_cell(
        &self,
        address: Relocatable,
        memory: &Memory,
    ) -> Result<Option<MaybeRelocatable>, RunnerError> {
        let index = address.offset % CELLS_PER_KECCAK as usize;
        if index < INPUT_CELLS_PER_KECCAK as usize {
            return Ok(None);
        }
        if let Some(felt) = self.cache.borrow().get(&address) {
            return Ok(Some(felt.into()));
        }
        let first_input_addr = (address - index)?;
        let first_output_addr = (first_input_addr + INPUT_CELLS_PER_KECCAK as usize)?;

        let mut input_felts = vec![];

        for i in 0..INPUT_CELLS_PER_KECCAK as usize {
            let m_index = (first_input_addr + i)?;
            let val = match memory.get(&m_index) {
                Some(value) => {
                    let num = value
                        .get_int_ref()
                        .ok_or(RunnerError::BuiltinExpectedInteger(Box::new((
                            BuiltinName::keccak,
                            (first_input_addr + i)?,
                        ))))?;
                    if num >= &KECCAK_INPUT_MAX {
                        return Err(RunnerError::IntegerBiggerThanPowerOfTwo(Box::new((
                            (first_input_addr + i)?,
                            BITS,
                            *num,
                        ))));
                    }
                    *num
                }
                _ => return Ok(None),
            };
            input_felts.push(val)
        }
        let input_message: Vec<u8> = input_felts
            .iter()
            .flat_map(|x| {
                let mut bytes = x.to_bytes_le().to_vec();
                bytes.resize(KECCAK_FELT_BYTE_SIZE, 0);
                bytes
            })
            .collect();
        let keccak_result = Self::keccak_f(&input_message)?;

        let mut start_index = 0_usize;
        for i in 0..INPUT_CELLS_PER_KECCAK {
            let end_index = start_index + BITS as usize / 8;
            self.cache.borrow_mut().insert((first_output_addr + i)?, {
                let mut bytes = keccak_result[start_index..end_index].to_vec();
                bytes.resize(32, 0);
                Felt252::from_bytes_le_slice(&bytes)
            });
            start_index = end_index;
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
        Ok(div_ceil(used_cells, CELLS_PER_KECCAK as usize))
    }

    pub fn get_used_diluted_check_units(&self, diluted_n_bits: u32) -> usize {
        // The diluted cells are:
        // state - 25 rounds times 1600 elements.
        // parity - 24 rounds times 1600/5 elements times 3 auxiliaries.
        // after_theta_rho_pi - 24 rounds times 1600 elements.
        // theta_aux - 24 rounds times 1600 elements.
        // chi_iota_aux - 24 rounds times 1600 elements times 2 auxiliaries.
        // In total 25 * 1600 + 24 * 320 * 3 + 24 * 1600 + 24 * 1600 + 24 * 1600 * 2 = 216640.
        // But we actually allocate 4 virtual columns, of dimensions 64 * 1024, in which we embed the
        // real cells, and we don't free the unused ones.
        // So the real number is 4 * 64 * 1024 = 262144.
        safe_div_usize(262144_usize, diluted_n_bits as usize).unwrap_or(0)
    }

    fn keccak_f(input_message: &[u8]) -> Result<Vec<u8>, RunnerError> {
        let bigint = BigUint::from_bytes_le(input_message);
        let mut keccak_input = bigint.to_u64_digits();
        keccak_input.resize(25, 0);
        // This unwrap wont fail as keccak_input's size is always 25
        let mut keccak_input: [u64; 25] = keccak_input.try_into().unwrap();
        keccak::f1600(&mut keccak_input);
        Ok(keccak_input.iter().flat_map(|x| x.to_le_bytes()).collect())
    }

    pub fn air_private_input(&self, memory: &Memory) -> Vec<PrivateInput> {
        let mut private_inputs = vec![];
        if let Some(segment) = memory.data.get(self.base) {
            let segment_len = segment.len();
            for (index, off) in (0..segment_len)
                .step_by(CELLS_PER_KECCAK as usize)
                .enumerate()
            {
                // Add the input cells of each keccak instance to the private inputs
                if let (
                    Ok(input_s0),
                    Ok(input_s1),
                    Ok(input_s2),
                    Ok(input_s3),
                    Ok(input_s4),
                    Ok(input_s5),
                    Ok(input_s6),
                    Ok(input_s7),
                ) = (
                    memory.get_integer((self.base as isize, off).into()),
                    memory.get_integer((self.base as isize, off + 1).into()),
                    memory.get_integer((self.base as isize, off + 2).into()),
                    memory.get_integer((self.base as isize, off + 3).into()),
                    memory.get_integer((self.base as isize, off + 4).into()),
                    memory.get_integer((self.base as isize, off + 5).into()),
                    memory.get_integer((self.base as isize, off + 6).into()),
                    memory.get_integer((self.base as isize, off + 7).into()),
                ) {
                    private_inputs.push(PrivateInput::KeccakState(PrivateInputKeccakState {
                        index,
                        input_s0: *input_s0,
                        input_s1: *input_s1,
                        input_s2: *input_s2,
                        input_s3: *input_s3,
                        input_s4: *input_s4,
                        input_s5: *input_s5,
                        input_s6: *input_s6,
                        input_s7: *input_s7,
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
    use crate::types::program::Program;
    use crate::utils::test_utils::*;
    use crate::{felt_hex, relocatable};

    use crate::vm::{
        errors::{memory_errors::MemoryError, runner_errors::RunnerError},
        runners::builtin_runner::BuiltinRunner,
    };

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_instances() {
        let builtin: BuiltinRunner = KeccakBuiltinRunner::new(Some(10), true).into();

        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![1]);

        assert_eq!(builtin.get_used_instances(&vm.segments), Ok(1));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack() {
        let mut builtin: BuiltinRunner = KeccakBuiltinRunner::new(Some(10), true).into();

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
        let mut builtin: BuiltinRunner = KeccakBuiltinRunner::new(Some(10), true).into();

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![992]);

        let pointer = Relocatable::from((2, 2));
        assert_eq!(
            builtin.final_stack(&vm.segments, pointer),
            Err(RunnerError::InvalidStopPointer(Box::new((
                BuiltinName::keccak,
                relocatable!(0, 992),
                relocatable!(0, 0)
            ))))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_when_not_included() {
        let mut builtin: BuiltinRunner = KeccakBuiltinRunner::new(Some(10), false).into();

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
        let mut builtin: BuiltinRunner = KeccakBuiltinRunner::new(Some(10), true).into();

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
            Err(RunnerError::NoStopPointer(Box::new(BuiltinName::keccak)))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_and_allocated_size_test() {
        let builtin: BuiltinRunner = KeccakBuiltinRunner::new(Some(10), true).into();

        let program = Program::from_bytes(
            include_bytes!("../../../../../cairo_programs/keccak.json"),
            Some("main"),
        )
        .unwrap();

        let mut cairo_runner = cairo_runner!(program);
        cairo_runner.vm.segments.segment_used_sizes = Some(vec![0]);

        let mut hint_processor = BuiltinHintProcessor::new_empty();

        let address = cairo_runner.initialize(false).unwrap();

        cairo_runner
            .run_until_pc(address, &mut hint_processor)
            .unwrap();

        assert_eq!(
            builtin.get_used_cells_and_allocated_size(&cairo_runner.vm),
            Ok((0, 1072))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units() {
        let builtin: BuiltinRunner = KeccakBuiltinRunner::new(Some(10), true).into();

        let mut vm = vm!();
        vm.current_step = 160;

        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(256));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_missing_segment_used_sizes() {
        let builtin: BuiltinRunner = KeccakBuiltinRunner::new(Some(2048), true).into();
        let vm = vm!();

        assert_eq!(
            builtin.get_used_cells(&vm.segments),
            Err(MemoryError::MissingSegmentUsedSizes)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_empty() {
        let builtin: BuiltinRunner = KeccakBuiltinRunner::new(Some(2048), true).into();
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells() {
        let builtin: BuiltinRunner = KeccakBuiltinRunner::new(Some(2048), true).into();
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(4));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initial_stackincluded_test() {
        let keccak_builtin = KeccakBuiltinRunner::new(Some(2048), true);
        assert_eq!(
            keccak_builtin.initial_stack(),
            vec![mayberelocatable!(0, 0)]
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initial_stack_notincluded_test() {
        let keccak_builtin = KeccakBuiltinRunner::new(Some(2048), false);
        assert_eq!(keccak_builtin.initial_stack(), Vec::new())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_memory_valid() {
        let memory = memory![
            ((0, 16), 43),
            ((0, 17), 199),
            ((0, 18), 0),
            ((0, 19), 0),
            ((0, 20), 0),
            ((0, 21), 0),
            ((0, 22), 0),
            ((0, 23), 1),
            ((0, 24), 0),
            ((0, 25), 0),
            ((0, 26), 43),
            ((0, 27), 199),
            ((0, 28), 0),
            ((0, 29), 0),
            ((0, 30), 0),
            ((0, 31), 0),
            ((0, 32), 0),
            ((0, 33), 1),
            ((0, 34), 0),
            ((0, 35), 0)
        ];
        let builtin = KeccakBuiltinRunner::new(Some(2048), true);

        let result = builtin.deduce_memory_cell(Relocatable::from((0, 25)), &memory);
        assert_eq!(
            result,
            Ok(Some(MaybeRelocatable::from(felt_hex!(
                "0xa06bd018ba91b93146f53563cff2efba46fee2eabe9d89b4cb"
            ))))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_non_reloc_address_err() {
        let memory = memory![
            ((0, 4), 32),
            ((0, 5), 72),
            ((0, 6), 0),
            ((0, 7), 120),
            ((0, 8), 52)
        ];
        let builtin = KeccakBuiltinRunner::new(Some(2048), true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 1)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_offset_lt_input_cell_length_none() {
        let memory = memory![((0, 4), 32)];
        let builtin = KeccakBuiltinRunner::new(Some(2048), true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 2)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_expected_integer() {
        let memory = memory![((0, 0), (1, 2))];

        let builtin = KeccakBuiltinRunner::new(Some(2048), true);

        let result = builtin.deduce_memory_cell(Relocatable::from((0, 9)), &memory);

        assert_eq!(
            result,
            Err(RunnerError::BuiltinExpectedInteger(Box::new((
                BuiltinName::keccak,
                (0, 0).into()
            ))))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_missing_input_cells() {
        let memory = memory![((0, 1), (1, 2))];

        let builtin = KeccakBuiltinRunner::new(Some(2048), true);

        let result = builtin.deduce_memory_cell(Relocatable::from((0, 1)), &memory);

        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_get_memory_err() {
        let memory = memory![((0, 35), 0)];

        let builtin = KeccakBuiltinRunner::new(Some(2048), true);

        let result = builtin.deduce_memory_cell(Relocatable::from((0, 15)), &memory);

        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_memory_int_larger_than_bits() {
        let mut memory = memory![
            ((0, 17), 199),
            ((0, 18), 0),
            ((0, 19), 0),
            ((0, 20), 0),
            ((0, 21), 0),
            ((0, 22), 0),
            ((0, 23), 1),
            ((0, 24), 0),
            ((0, 25), 0),
            ((0, 26), 43),
            ((0, 27), 199),
            ((0, 28), 0),
            ((0, 29), 0),
            ((0, 30), 0),
            ((0, 31), 0),
            ((0, 32), 0),
            ((0, 33), 1),
            ((0, 34), 0),
            ((0, 35), 0)
        ];

        memory.insert((0, 16).into(), Felt252::MAX).unwrap();

        let builtin = KeccakBuiltinRunner::new(Some(2048), true);

        let result = builtin.deduce_memory_cell(Relocatable::from((0, 25)), &memory);

        assert_eq!(
            result,
            Err(RunnerError::IntegerBiggerThanPowerOfTwo(Box::new((
                (0, 16).into(),
                BITS,
                Felt252::MAX
            ))))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_diluted_check_units_result() {
        let builtin = KeccakBuiltinRunner::new(Some(2048), true);

        let result: usize = builtin.get_used_diluted_check_units(16);

        assert_eq!(result, 16384);
    }

    #[test]
    fn keccak_f() {
        let input_bytes = b"\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x03\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x04\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x05\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x06\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x07\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x08\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let expected_output_bytes = b"\xf6\x98\x81\xe1\x00!\x1f.\xc4*\x8c\x0c\x7fF\xc8q8\xdf\xb9\xbe\x07H\xca7T1\xab\x16\x17\xa9\x11\xff-L\x87\xb2iY.\x96\x82x\xde\xbb\\up?uz:0\xee\x08\x1b\x15\xd6\n\xab\r\x0b\x87T:w\x0fH\xe7!f},\x08a\xe5\xbe8\x16\x13\x9a?\xad~<9\xf7\x03`\x8b\xd8\xa3F\x8aQ\xf9\n9\xcdD\xb7.X\xf7\x8e\x1f\x17\x9e \xe5i\x01rr\xdf\xaf\x99k\x9f\x8e\x84\\\xday`\xf1``\x02q+\x8e\xad\x96\xd8\xff\xff3<\xb6\x01o\xd7\xa6\x86\x9d\xea\xbc\xfb\x08\xe1\xa3\x1c\x06z\xab@\xa1\xc1\xb1xZ\x92\x96\xc0.\x01\x13g\x93\x87!\xa6\xa8z\x9c@\x0bY'\xe7\xa7Qr\xe5\xc1\xa3\xa6\x88H\xa5\xc0@9k:y\xd1Kw\xd5";
        let output_bytes = KeccakBuiltinRunner::keccak_f(input_bytes);
        assert_eq!(output_bytes, Ok(expected_output_bytes.to_vec()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_air_private_input() {
        let builtin: BuiltinRunner = KeccakBuiltinRunner::new(Some(2048), true).into();

        let segments = segments![
            ((0, 0), 0),
            ((0, 1), 1),
            ((0, 2), 2),
            ((0, 3), 3),
            ((0, 4), 4),
            ((0, 5), 5),
            ((0, 6), 6),
            ((0, 7), 7)
        ];
        assert_eq!(
            builtin.air_private_input(&segments),
            (vec![PrivateInput::KeccakState(PrivateInputKeccakState {
                index: 0,
                input_s0: 0.into(),
                input_s1: 1.into(),
                input_s2: 2.into(),
                input_s3: 3.into(),
                input_s4: 4.into(),
                input_s5: 5.into(),
                input_s6: 6.into(),
                input_s7: 7.into()
            }),]),
        );
    }
}
