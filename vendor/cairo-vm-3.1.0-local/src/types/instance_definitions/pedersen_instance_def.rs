use serde::Serialize;

pub(crate) const CELLS_PER_HASH: u32 = 3;
pub(crate) const INPUT_CELLS_PER_HASH: u32 = 2;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub(crate) struct PedersenInstanceDef {
    pub(crate) ratio: Option<u32>,
}

impl Default for PedersenInstanceDef {
    fn default() -> Self {
        PedersenInstanceDef { ratio: Some(8) }
    }
}

impl PedersenInstanceDef {
    pub(crate) fn new(ratio: Option<u32>) -> Self {
        PedersenInstanceDef { ratio }
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
        let builtin_instance = PedersenInstanceDef { ratio: Some(10) };
        assert_eq!(PedersenInstanceDef::new(Some(10)), builtin_instance);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_default() {
        let builtin_instance = PedersenInstanceDef { ratio: Some(8) };
        assert_eq!(PedersenInstanceDef::default(), builtin_instance);
    }
}
