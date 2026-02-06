use crate::tests::run_program_simple;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

#[test]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn skip_next_instruction_test() {
    let program_data = include_bytes!(
        "../../../cairo_programs/noretrocompat/test_skip_next_instruction.noretrocompat.json"
    );
    run_program_simple(program_data.as_slice());
}
