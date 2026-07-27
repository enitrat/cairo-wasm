use std::collections::BTreeMap;

use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_compiler::project::InMemoryProject;
use cairo_lang_compiler::{CompilerConfig, compile_in_memory_project};
use cairo_lang_lowering::utils::InliningStrategy;
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

include!(concat!(env!("OUT_DIR"), "/embedded_corelib.rs"));

#[derive(Debug, Deserialize)]
pub struct CompileRequest {
    pub crate_name: String,
    pub files: BTreeMap<String, String>,
    #[serde(default)]
    pub corelib_files: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub replace_ids: bool,
    #[serde(default)]
    pub inlining_strategy: InliningStrategyArg,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum InliningStrategyArg {
    #[default]
    Default,
    Avoid,
}

#[derive(Debug, Serialize)]
pub struct CompileResponse {
    pub success: bool,
    pub sierra: Option<String>,
    pub diagnostics: String,
    pub error: Option<String>,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn compile(request_json: &str) -> String {
    let request: CompileRequest = match serde_json::from_str(request_json) {
        Ok(request) => request,
        Err(error) => {
            return serde_json::to_string(&CompileResponse {
                success: false,
                sierra: None,
                diagnostics: String::new(),
                error: Some(format!("Failed parsing request JSON: {error}")),
            })
            .expect("serialize error response");
        }
    };

    let mut diagnostics = String::new();
    let compiler_config = CompilerConfig {
        diagnostics_reporter: DiagnosticsReporter::write_to_string(&mut diagnostics),
        replace_ids: request.replace_ids,
        ..CompilerConfig::default()
    };

    let project = InMemoryProject {
        main_crate_name: request.crate_name,
        main_crate_files: request.files,
        corelib_files: request.corelib_files.unwrap_or_else(embedded_corelib_files),
        main_crate_settings: None,
    };

    let inlining_strategy = match request.inlining_strategy {
        InliningStrategyArg::Default => InliningStrategy::Default,
        InliningStrategyArg::Avoid => InliningStrategy::Avoid,
    };

    let response = match compile_in_memory_project(&project, compiler_config, inlining_strategy) {
        Ok(program) => CompileResponse {
            success: true,
            sierra: Some(program.to_string()),
            diagnostics,
            error: None,
        },
        Err(error) => CompileResponse {
            success: false,
            sierra: None,
            diagnostics,
            error: Some(error.to_string()),
        },
    };

    serde_json::to_string(&response).expect("serialize compile response")
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn embedded_corelib_manifest() -> String {
    let files =
        EMBEDDED_CORELIB_FILES.iter().map(|(path, _)| (*path).to_string()).collect::<Vec<_>>();
    serde_json::to_string(&files).expect("serialize corelib manifest")
}

fn embedded_corelib_files() -> BTreeMap<String, String> {
    EMBEDDED_CORELIB_FILES
        .iter()
        .map(|(path, content)| ((*path).to_string(), (*content).to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::compile;

    #[test]
    fn compile_executable_program() {
        let request = json!({
            "crate_name": "test",
            "files": {
                "lib.cairo": "#[executable]\nfn main() { println!(\"Hello executable\"); }"
            },
            "replace_ids": true
        });

        let response = compile(&request.to_string());
        let response_json: Value = serde_json::from_str(&response).expect("valid JSON response");

        assert_eq!(response_json["success"], true, "response={response}");
        assert_eq!(response_json["error"], Value::Null);
        assert!(response_json["sierra"].is_string());
    }
}
