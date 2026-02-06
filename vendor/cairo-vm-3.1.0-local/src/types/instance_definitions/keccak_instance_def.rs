use crate::stdlib::prelude::*;
use serde::Serialize;

pub(crate) const INPUT_CELLS_PER_KECCAK: u32 = 8;
pub(crate) const CELLS_PER_KECCAK: u32 = 16;
pub(crate) const KECCAK_INSTANCES_PER_COMPONENT: u32 = 16;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub(crate) struct KeccakInstanceDef {
    pub(crate) ratio: Option<u32>,
}

impl Default for KeccakInstanceDef {
    fn default() -> Self {
        // ratio should be equal to 2 ** 11 -> 2048
        KeccakInstanceDef { ratio: Some(2048) }
    }
}

impl KeccakInstanceDef {
    pub(crate) fn new(ratio: Option<u32>) -> Self {
        KeccakInstanceDef { ratio }
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
        let builtin_instance = KeccakInstanceDef { ratio: Some(2048) };
        assert_eq!(KeccakInstanceDef::new(Some(2048)), builtin_instance);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_default() {
        let builtin_instance = KeccakInstanceDef { ratio: Some(2048) };
        assert_eq!(KeccakInstanceDef::default(), builtin_instance);
    }
}
