use crate::stdlib::{collections::BTreeMap, prelude::*};
use crate::types::builtin_name::BuiltinName;
use crate::types::relocatable::{MaybeRelocatable, Relocatable};
use crate::vm::errors::memory_errors::MemoryError;
use crate::vm::errors::runner_errors::RunnerError;
use crate::vm::runners::cairo_pie::{
    Attributes, BuiltinAdditionalData, OutputBuiltinAdditionalData, Pages, PublicMemoryPage,
};
use crate::vm::vm_core::VirtualMachine;
use crate::vm::vm_memory::memory_segments::MemorySegmentManager;

#[derive(Debug, Clone, PartialEq)]
pub struct OutputBuiltinState {
    pub base: usize,
    pub base_offset: usize,
    pub pages: Pages,
    pub attributes: Attributes,
}

#[derive(Debug, Clone)]
pub struct OutputBuiltinRunner {
    base: usize,
    pub base_offset: usize,
    pub(crate) pages: Pages,
    pub(crate) attributes: Attributes,
    pub(crate) stop_ptr: Option<usize>,
    pub(crate) included: bool,
}

impl OutputBuiltinRunner {
    pub fn new(included: bool) -> OutputBuiltinRunner {
        OutputBuiltinRunner {
            base: 0,
            base_offset: 0,
            pages: BTreeMap::default(),
            attributes: BTreeMap::default(),
            stop_ptr: None,
            included,
        }
    }

    pub fn new_state(&mut self, base: usize, base_offset: usize, included: bool) {
        self.base = base;
        self.base_offset = base_offset;
        self.pages = BTreeMap::default();
        self.attributes = BTreeMap::default();
        self.stop_ptr = None;
        self.included = included;
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

    pub fn get_allocated_memory_units(&self, _vm: &VirtualMachine) -> Result<usize, MemoryError> {
        Ok(0)
    }

    pub fn get_used_cells(&self, segments: &MemorySegmentManager) -> Result<usize, MemoryError> {
        segments
            .get_segment_used_size(self.base)
            .ok_or(MemoryError::MissingSegmentUsedSizes)
    }

    pub fn get_used_instances(
        &self,
        segments: &MemorySegmentManager,
    ) -> Result<usize, MemoryError> {
        self.get_used_cells(segments)
    }

    pub fn final_stack(
        &mut self,
        segments: &MemorySegmentManager,
        pointer: Relocatable,
    ) -> Result<Relocatable, RunnerError> {
        if self.included {
            let stop_pointer_addr = (pointer - 1)
                .map_err(|_| RunnerError::NoStopPointer(Box::new(BuiltinName::output)))?;
            let stop_pointer = segments
                .memory
                .get_relocatable(stop_pointer_addr)
                .map_err(|_| RunnerError::NoStopPointer(Box::new(BuiltinName::output)))?;
            if self.base as isize != stop_pointer.segment_index {
                return Err(RunnerError::InvalidStopPointerIndex(Box::new((
                    BuiltinName::output,
                    stop_pointer,
                    self.base,
                ))));
            }
            let stop_ptr = stop_pointer.offset;
            let used = self.get_used_cells(segments).map_err(RunnerError::Memory)?;
            if stop_ptr != used {
                return Err(RunnerError::InvalidStopPointer(Box::new((
                    BuiltinName::output,
                    Relocatable::from((self.base as isize, used)),
                    Relocatable::from((self.base as isize, stop_ptr)),
                ))));
            }
            self.stop_ptr = Some(stop_ptr);
            Ok(stop_pointer_addr)
        } else {
            self.stop_ptr = Some(0);
            Ok(pointer)
        }
    }

    pub fn add_attribute(&mut self, name: String, value: Vec<usize>) {
        self.attributes.insert(name, value);
    }

    pub fn get_additional_data(&self) -> BuiltinAdditionalData {
        BuiltinAdditionalData::Output(OutputBuiltinAdditionalData {
            pages: self.pages.clone(),
            attributes: self.attributes.clone(),
        })
    }

    pub fn extend_additional_data(
        &mut self,
        additional_data: &BuiltinAdditionalData,
    ) -> Result<(), RunnerError> {
        let additional_data = match additional_data {
            BuiltinAdditionalData::Output(d) => d,
            _ => return Err(RunnerError::InvalidAdditionalData(BuiltinName::output)),
        };
        self.pages.extend(additional_data.pages.clone());
        self.attributes.extend(additional_data.attributes.clone());
        Ok(())
    }

    pub(crate) fn set_stop_ptr_offset(&mut self, offset: usize) {
        self.stop_ptr = Some(offset)
    }

    pub fn set_state(&mut self, new_state: OutputBuiltinState) {
        self.base = new_state.base;
        self.base_offset = new_state.base_offset;
        self.pages = new_state.pages;
        self.attributes = new_state.attributes;
    }

    pub fn get_state(&mut self) -> OutputBuiltinState {
        OutputBuiltinState {
            base: self.base,
            base_offset: self.base_offset,
            pages: self.pages.clone(),
            attributes: self.attributes.clone(),
        }
    }

    pub fn add_page(
        &mut self,
        page_id: usize,
        page_start: Relocatable,
        page_size: usize,
    ) -> Result<(), RunnerError> {
        if page_start.segment_index as usize != self.base {
            return Err(RunnerError::PageNotOnSegment(page_start, self.base));
        }

        self.pages.insert(
            page_id,
            PublicMemoryPage {
                start: page_start.offset - self.base_offset,
                size: page_size,
            },
        );

        Ok(())
    }

    pub fn get_public_memory(
        &self,
        segments: &MemorySegmentManager,
    ) -> Result<Vec<(usize, usize)>, RunnerError> {
        let size = self.get_used_cells(segments)?;

        let mut public_memory: Vec<(usize, usize)> = (0..size).map(|i| (i, 0)).collect();
        for (page_id, page) in self.pages.iter() {
            for index in 0..page.size {
                public_memory[page.start + index].1 = *page_id;
            }
        }

        Ok(public_memory)
    }
}

impl Default for OutputBuiltinRunner {
    fn default() -> Self {
        Self::new(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relocatable;

    use crate::{
        utils::test_utils::*,
        vm::{errors::memory_errors::MemoryError, runners::builtin_runner::BuiltinRunner},
    };

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_instances() {
        let builtin = OutputBuiltinRunner::new(true);

        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![1]);

        assert_eq!(builtin.get_used_instances(&vm.segments), Ok(1));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack() {
        let mut builtin = OutputBuiltinRunner::new(true);

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
        let mut builtin = OutputBuiltinRunner::new(true);

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![998]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer),
            Err(RunnerError::InvalidStopPointer(Box::new((
                BuiltinName::output,
                relocatable!(0, 998),
                relocatable!(0, 0)
            ))))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_when_notincluded() {
        let mut builtin = OutputBuiltinRunner::new(false);

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
        let mut builtin = OutputBuiltinRunner::new(true);

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
            Err(RunnerError::NoStopPointer(Box::new(BuiltinName::output)))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_and_allocated_size_test() {
        let builtin: BuiltinRunner = OutputBuiltinRunner::new(true).into();

        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);

        assert_eq!(
            builtin.get_used_cells_and_allocated_size(&vm),
            Ok((0_usize, 0))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units() {
        let builtin = OutputBuiltinRunner::new(true);

        let vm = vm!();

        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_segments_for_output() {
        let mut builtin = OutputBuiltinRunner::new(true);
        let mut segments = MemorySegmentManager::new();
        builtin.initialize_segments(&mut segments);
        assert_eq!(builtin.base, 0);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_initial_stack_for_output_with_base() {
        let mut builtin = OutputBuiltinRunner::new(true);
        builtin.base = 1;
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
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let vm = vm!();

        assert_eq!(
            builtin.get_used_cells(&vm.segments),
            Err(MemoryError::MissingSegmentUsedSizes)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_empty() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(4));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_get_used_instances_missing_segments() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let memory_segment_manager = MemorySegmentManager::new();

        assert_eq!(
            builtin.get_used_instances(&memory_segment_manager),
            Err(MemoryError::MissingSegmentUsedSizes)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_get_used_instances_valid() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let mut memory_segment_manager = MemorySegmentManager::new();
        memory_segment_manager.segment_used_sizes = Some(vec![0]);

        assert_eq!(builtin.get_used_instances(&memory_segment_manager), Ok(0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_deduce_memory_cell_output_builtin() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
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
            builtin.deduce_memory_cell(pointer, &vm.segments.memory),
            Ok(None)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_add_validation_rule() {
        let builtin: BuiltinRunner = OutputBuiltinRunner::new(true).into();
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
    fn get_additional_data_no_pages_no_attributes() {
        let builtin = OutputBuiltinRunner::new(true);
        assert_eq!(
            builtin.get_additional_data(),
            BuiltinAdditionalData::Output(OutputBuiltinAdditionalData {
                pages: BTreeMap::default(),
                attributes: BTreeMap::default()
            })
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_air_private_input() {
        let builtin: BuiltinRunner = OutputBuiltinRunner::new(true).into();

        let segments = segments![((0, 0), 0), ((0, 1), 1), ((0, 2), 2), ((0, 3), 3)];
        assert!(builtin.air_private_input(&segments).is_empty());
    }

    #[test]
    fn set_state() {
        let mut builtin = OutputBuiltinRunner::new(true);
        assert_eq!(builtin.base, 0);

        let new_state = OutputBuiltinState {
            base: 10,
            base_offset: 0,
            pages: BTreeMap::from([(1, PublicMemoryPage { start: 0, size: 3 })]),
            attributes: BTreeMap::from([("gps_fact_topology".to_string(), vec![0, 2, 0])]),
        };
        builtin.set_state(new_state.clone());

        assert_eq!(builtin.base, new_state.base);
        assert_eq!(builtin.pages, new_state.pages);
        assert_eq!(builtin.attributes, new_state.attributes);

        let state = builtin.get_state();
        assert_eq!(state, new_state);
    }

    #[test]
    fn new_state() {
        let mut builtin = OutputBuiltinRunner {
            base: 10,
            base_offset: 0,
            pages: BTreeMap::from([(1, PublicMemoryPage { start: 0, size: 3 })]),
            attributes: BTreeMap::from([("gps_fact_topology".to_string(), vec![0, 2, 0])]),
            stop_ptr: Some(10),
            included: true,
        };

        let new_base = 11;
        let new_included = false;
        builtin.new_state(new_base, 2, new_included);

        assert_eq!(builtin.base, new_base);
        assert_eq!(builtin.base_offset, 2);
        assert!(builtin.pages.is_empty());
        assert!(builtin.attributes.is_empty());
        assert_eq!(builtin.stop_ptr, None);
        assert_eq!(builtin.included, new_included);
    }

    #[test]
    fn add_page() {
        let mut builtin = OutputBuiltinRunner::new(true);
        assert_eq!(
            builtin.add_page(
                1,
                Relocatable {
                    segment_index: builtin.base() as isize,
                    offset: 0
                },
                3
            ),
            Ok(())
        );

        assert_eq!(
            builtin.pages,
            BTreeMap::from([(1, PublicMemoryPage { start: 0, size: 3 }),])
        )
    }

    #[test]
    fn add_page_wrong_segment() {
        let mut builtin = OutputBuiltinRunner::new(true);
        let page_start = Relocatable {
            segment_index: 18,
            offset: 0,
        };

        let result = builtin.add_page(1, page_start, 3);
        assert!(
            matches!(result, Err(RunnerError::PageNotOnSegment(relocatable, base)) if relocatable == page_start && base == builtin.base())
        )
    }

    #[test]
    pub fn add_attribute() {
        let mut builtin = OutputBuiltinRunner::new(true);
        assert!(builtin.attributes.is_empty());

        let name = "gps_fact_topology".to_string();
        let values = vec![0, 12, 30];
        builtin.add_attribute(name.clone(), values.clone());

        assert_eq!(builtin.attributes, BTreeMap::from([(name, values)]));
    }

    #[test]
    fn get_public_memory() {
        let mut builtin = OutputBuiltinRunner::new(true);

        builtin
            .add_page(
                1,
                Relocatable {
                    segment_index: builtin.base() as isize,
                    offset: 2,
                },
                2,
            )
            .unwrap();

        builtin
            .add_page(
                2,
                Relocatable {
                    segment_index: builtin.base() as isize,
                    offset: 4,
                },
                3,
            )
            .unwrap();

        let mut segments = MemorySegmentManager::new();
        segments.segment_used_sizes = Some(vec![7]);

        let public_memory = builtin.get_public_memory(&segments).unwrap();
        assert_eq!(
            public_memory,
            vec![(0, 0), (1, 0), (2, 1), (3, 1), (4, 2), (5, 2), (6, 2)]
        );
    }

    #[test]
    fn get_and_extend_additional_data() {
        let builtin_a = OutputBuiltinRunner {
            base: 0,
            base_offset: 0,
            pages: BTreeMap::from([(1, PublicMemoryPage { start: 0, size: 3 })]),
            attributes: BTreeMap::from([("gps_fact_topology".to_string(), vec![0, 2, 0])]),
            stop_ptr: None,
            included: true,
        };
        let additional_data = builtin_a.get_additional_data();
        let mut builtin_b = OutputBuiltinRunner {
            base: 0,
            base_offset: 0,
            pages: Default::default(),
            attributes: Default::default(),
            stop_ptr: None,
            included: true,
        };
        builtin_b.extend_additional_data(&additional_data).unwrap();
        assert_eq!(builtin_a.attributes, builtin_b.attributes);
        assert_eq!(builtin_a.pages, builtin_b.pages);
    }
}
