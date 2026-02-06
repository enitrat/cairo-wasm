use serde::Serialize;

pub(crate) const CELLS_PER_POSEIDON: u32 = 6;
pub(crate) const INPUT_CELLS_PER_POSEIDON: u32 = 3;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub(crate) struct PoseidonInstanceDef {
    pub(crate) ratio: Option<u32>,
}

impl Default for PoseidonInstanceDef {
    fn default() -> Self {
        PoseidonInstanceDef { ratio: Some(32) }
    }
}
impl PoseidonInstanceDef {
    pub(crate) fn new(ratio: Option<u32>) -> Self {
        PoseidonInstanceDef { ratio }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let builtin_instance = PoseidonInstanceDef { ratio: Some(8) };
        assert_eq!(PoseidonInstanceDef::new(Some(8)), builtin_instance);
    }

    #[test]
    fn test_default() {
        let builtin_instance = PoseidonInstanceDef { ratio: Some(32) };
        assert_eq!(PoseidonInstanceDef::default(), builtin_instance);
    }
}
