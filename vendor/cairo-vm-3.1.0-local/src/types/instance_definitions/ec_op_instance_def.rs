use serde::Serialize;

pub(crate) const CELLS_PER_EC_OP: u32 = 7;
pub(crate) const INPUT_CELLS_PER_EC_OP: u32 = 5;
pub(crate) const SCALAR_HEIGHT: u32 = 256;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub(crate) struct EcOpInstanceDef {
    pub(crate) ratio: Option<u32>,
}

impl Default for EcOpInstanceDef {
    fn default() -> Self {
        EcOpInstanceDef { ratio: Some(256) }
    }
}

impl EcOpInstanceDef {
    pub(crate) fn new(ratio: Option<u32>) -> Self {
        EcOpInstanceDef { ratio }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_new() {
        let builtin_instance = EcOpInstanceDef { ratio: Some(8) };
        assert_eq!(EcOpInstanceDef::new(Some(8)), builtin_instance);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_default() {
        let builtin_instance = EcOpInstanceDef { ratio: Some(256) };
        assert_eq!(EcOpInstanceDef::default(), builtin_instance);
    }
}
