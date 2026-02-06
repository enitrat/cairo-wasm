use serde::Serialize;

pub(crate) const CELLS_PER_BITWISE: u32 = 5;
pub(crate) const INPUT_CELLS_PER_BITWISE: u32 = 2;
pub(crate) const TOTAL_N_BITS: u32 = 251;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub(crate) struct BitwiseInstanceDef {
    pub(crate) ratio: Option<u32>,
}

impl Default for BitwiseInstanceDef {
    fn default() -> Self {
        BitwiseInstanceDef { ratio: Some(256) }
    }
}

impl BitwiseInstanceDef {
    pub(crate) fn new(ratio: Option<u32>) -> Self {
        BitwiseInstanceDef { ratio }
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
        let builtin_instance = BitwiseInstanceDef { ratio: Some(8) };
        assert_eq!(BitwiseInstanceDef::new(Some(8)), builtin_instance);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_default() {
        let builtin_instance = BitwiseInstanceDef { ratio: Some(256) };
        assert_eq!(BitwiseInstanceDef::default(), builtin_instance);
    }
}
