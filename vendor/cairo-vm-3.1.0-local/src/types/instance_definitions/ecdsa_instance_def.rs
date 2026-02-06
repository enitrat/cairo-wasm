use serde::Serialize;

pub(crate) const CELLS_PER_SIGNATURE: u32 = 2;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub(crate) struct EcdsaInstanceDef {
    pub(crate) ratio: Option<u32>,
}

impl Default for EcdsaInstanceDef {
    fn default() -> Self {
        EcdsaInstanceDef { ratio: Some(512) }
    }
}

impl EcdsaInstanceDef {
    pub(crate) fn new(ratio: Option<u32>) -> Self {
        EcdsaInstanceDef { ratio }
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
        let builtin_instance = EcdsaInstanceDef { ratio: Some(8) };
        assert_eq!(EcdsaInstanceDef::new(Some(8)), builtin_instance);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_default() {
        let builtin_instance = EcdsaInstanceDef { ratio: Some(512) };
        assert_eq!(EcdsaInstanceDef::default(), builtin_instance);
    }
}
