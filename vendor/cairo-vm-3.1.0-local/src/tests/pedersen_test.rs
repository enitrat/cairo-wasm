use crate::tests::run_program_with_trace;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

#[test]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pedersen_integration_test() {
    let program_data = include_bytes!("../../../cairo_programs/pedersen_test.json");
    let expected_trace = [
        (7, 25, 25),
        (8, 26, 25),
        (10, 27, 25),
        (12, 28, 25),
        (1, 30, 30),
        (2, 30, 30),
        (3, 30, 30),
        (5, 31, 30),
        (6, 32, 30),
        (14, 32, 25),
        (15, 32, 25),
        (17, 33, 25),
        (18, 34, 25),
        (19, 35, 25),
    ];
    run_program_with_trace(program_data.as_slice(), expected_trace.as_slice());
}
