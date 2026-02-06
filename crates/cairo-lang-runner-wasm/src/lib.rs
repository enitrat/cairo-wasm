use std::collections::BTreeMap;

use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_compiler::project::InMemoryProject;
use cairo_lang_compiler::{CompilerConfig, compile_in_memory_project};
use cairo_lang_lowering::utils::InliningStrategy;
use cairo_lang_runner::{RunResultValue, SierraCasmRunner, StarknetState};
use cairo_lang_sierra::ProgramParser;
use cairo_lang_sierra::program::Program;
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

include!(concat!(env!("OUT_DIR"), "/embedded_corelib.rs"));

#[derive(Debug, Deserialize)]
pub struct CompileAndRunRequest {
    pub crate_name: String,
    pub files: BTreeMap<String, String>,
    #[serde(default)]
    pub corelib_files: Option<BTreeMap<String, String>>,
    #[serde(default = "default_replace_ids")]
    pub replace_ids: bool,
    #[serde(default)]
    pub inlining_strategy: InliningStrategyArg,
    pub available_gas: Option<usize>,
    #[serde(default = "default_function_name")]
    pub function: String,
}

#[derive(Debug, Deserialize)]
pub struct RunSierraRequest {
    pub sierra: String,
    pub available_gas: Option<usize>,
    #[serde(default = "default_function_name")]
    pub function: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum InliningStrategyArg {
    #[default]
    Default,
    Avoid,
}

#[derive(Debug, Serialize)]
pub struct RunResponse {
    pub success: bool,
    pub panicked: bool,
    pub values: Vec<String>,
    pub stdout: String,
    pub gas_counter: Option<String>,
    pub diagnostics: String,
    pub error: Option<String>,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn compile_and_run(request_json: &str) -> String {
    let request: CompileAndRunRequest = match serde_json::from_str(request_json) {
        Ok(request) => request,
        Err(error) => {
            return serialize_error(String::new(), format!("Failed parsing request JSON: {error}"));
        }
    };

    let mut diagnostics = String::new();
    let compiler_config = CompilerConfig {
        diagnostics_reporter: DiagnosticsReporter::write_to_string(&mut diagnostics),
        replace_ids: request.replace_ids,
        ..CompilerConfig::default()
    };

    let inlining_strategy = match request.inlining_strategy {
        InliningStrategyArg::Default => InliningStrategy::Default,
        InliningStrategyArg::Avoid => InliningStrategy::Avoid,
    };
    let project = InMemoryProject {
        main_crate_name: request.crate_name,
        main_crate_files: request.files,
        corelib_files: request.corelib_files.unwrap_or_else(embedded_corelib_files),
        main_crate_settings: None,
    };

    let program = match compile_in_memory_project(&project, compiler_config, inlining_strategy) {
        Ok(program) => program,
        Err(error) => return serialize_error(diagnostics, error.to_string()),
    };

    serialize_run_response(run_program(
        program,
        &request.function,
        request.available_gas,
        diagnostics,
    ))
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn run_sierra(request_json: &str) -> String {
    let request: RunSierraRequest = match serde_json::from_str(request_json) {
        Ok(request) => request,
        Err(error) => {
            return serialize_error(String::new(), format!("Failed parsing request JSON: {error}"));
        }
    };

    let program = match ProgramParser::new().parse(&request.sierra) {
        Ok(program) => program,
        Err(error) => {
            return serialize_error(
                String::new(),
                format!("Failed parsing Sierra program: {error:?}"),
            );
        }
    };

    serialize_run_response(run_program(
        program,
        &request.function,
        request.available_gas,
        String::new(),
    ))
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn embedded_corelib_manifest() -> String {
    let files =
        EMBEDDED_CORELIB_FILES.iter().map(|(path, _)| (*path).to_string()).collect::<Vec<_>>();
    serde_json::to_string(&files).expect("serialize corelib manifest")
}

fn run_program(
    program: Program,
    function: &str,
    available_gas: Option<usize>,
    diagnostics: String,
) -> RunResponse {
    if available_gas.is_none() && program.requires_gas_counter() {
        return RunResponse {
            success: false,
            panicked: false,
            values: vec![],
            stdout: String::new(),
            gas_counter: None,
            diagnostics,
            error: Some("Program requires gas counter; provide `available_gas`.".into()),
        };
    }

    let runner = match SierraCasmRunner::new(
        program,
        if available_gas.is_some() { Some(Default::default()) } else { None },
        Default::default(),
        None,
    ) {
        Ok(runner) => runner,
        Err(error) => {
            return RunResponse {
                success: false,
                panicked: false,
                values: vec![],
                stdout: String::new(),
                gas_counter: None,
                diagnostics,
                error: Some(format!("Failed setting up runner: {error}")),
            };
        }
    };

    let func = match runner.find_function(function) {
        Ok(func) => func,
        Err(error) => {
            return RunResponse {
                success: false,
                panicked: false,
                values: vec![],
                stdout: String::new(),
                gas_counter: None,
                diagnostics,
                error: Some(format!("Failed finding function `{function}`: {error}")),
            };
        }
    };

    let result = match runner.run_function_with_starknet_context(
        func,
        vec![],
        available_gas,
        StarknetState::default(),
    ) {
        Ok(result) => result,
        Err(error) => {
            return RunResponse {
                success: false,
                panicked: false,
                values: vec![],
                stdout: String::new(),
                gas_counter: None,
                diagnostics,
                error: Some(format!("Failed to run function `{function}`: {error}")),
            };
        }
    };

    let (panicked, values) = match result.value {
        RunResultValue::Success(values) => (false, values),
        RunResultValue::Panic(values) => (true, values),
    };

    RunResponse {
        success: !panicked,
        panicked,
        values: values.into_iter().map(|felt| felt.to_string()).collect(),
        stdout: result.stdout,
        gas_counter: result.gas_counter.map(|gas| gas.to_string()),
        diagnostics,
        error: None,
    }
}

fn default_function_name() -> String {
    "::main".into()
}

fn default_replace_ids() -> bool {
    true
}

fn embedded_corelib_files() -> BTreeMap<String, String> {
    EMBEDDED_CORELIB_FILES
        .iter()
        .map(|(path, content)| ((*path).to_string(), (*content).to_string()))
        .collect()
}

fn serialize_error(diagnostics: String, error: String) -> String {
    serialize_run_response(RunResponse {
        success: false,
        panicked: false,
        values: vec![],
        stdout: String::new(),
        gas_counter: None,
        diagnostics,
        error: Some(error),
    })
}

fn serialize_run_response(response: RunResponse) -> String {
    serde_json::to_string(&response).expect("serialize run response")
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::compile_and_run;

    #[test]
    fn compile_and_run_simple_program() {
        let request = json!({
            "crate_name": "test",
            "files": {
                "lib.cairo": "fn main() -> felt252 { 7 }"
            },
            "available_gas": 1000000
        });

        let response = compile_and_run(&request.to_string());
        let response_json: Value = serde_json::from_str(&response).expect("valid JSON response");

        assert_eq!(response_json["success"], true, "response={response}");
        assert_eq!(response_json["panicked"], false);
        assert_eq!(response_json["error"], Value::Null);
        assert_eq!(response_json["stdout"], "");
        assert_eq!(response_json["values"], json!(["7"]));
    }

    #[test]
    fn compile_and_run_hello_world() {
        let request = json!({
            "crate_name": "test",
            "files": {
                "lib.cairo": "fn main() { println!(\"Hello World\"); }"
            },
            "available_gas": 1000000
        });

        let response = compile_and_run(&request.to_string());
        let response_json: Value = serde_json::from_str(&response).expect("valid JSON response");

        assert_eq!(response_json["success"], true, "response={response}");
        assert_eq!(response_json["panicked"], false);
        assert_eq!(response_json["error"], Value::Null);
        assert_eq!(response_json["stdout"], "Hello World\n");
    }

    #[test]
    fn compile_and_run_executable_hello_world() {
        let request = json!({
            "crate_name": "test",
            "files": {
                "lib.cairo": "#[executable]\nfn main() { println!(\"Hello executable\"); }"
            },
            "available_gas": 1000000
        });

        let response = compile_and_run(&request.to_string());
        let response_json: Value = serde_json::from_str(&response).expect("valid JSON response");

        assert_eq!(response_json["success"], true, "response={response}");
        assert_eq!(response_json["panicked"], false);
        assert_eq!(response_json["error"], Value::Null);
        assert_eq!(response_json["stdout"], "Hello executable\n");
    }
}
