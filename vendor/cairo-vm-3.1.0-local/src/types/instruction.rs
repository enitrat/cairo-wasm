use crate::Felt252;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};

use crate::vm::decoding::decoder::decode_instruction;

#[cfg(feature = "test_utils")]
use arbitrary::Arbitrary;

#[cfg_attr(feature = "test_utils", derive(Arbitrary))]
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub enum Register {
    AP,
    FP,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Instruction {
    pub off0: isize,
    pub off1: isize,
    pub off2: isize,
    pub dst_register: Register,
    pub op0_register: Register,
    pub op1_addr: Op1Addr,
    pub res: Res,
    pub pc_update: PcUpdate,
    pub ap_update: ApUpdate,
    pub fp_update: FpUpdate,
    pub opcode: Opcode,
    pub opcode_extension: OpcodeExtension,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Op1Addr {
    Imm,
    AP,
    FP,
    Op0,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Res {
    Op1,
    Add,
    Mul,
    Unconstrained,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PcUpdate {
    Regular,
    Jump,
    JumpRel,
    Jnz,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ApUpdate {
    Regular,
    Add,
    Add1,
    Add2,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FpUpdate {
    Regular,
    APPlus2,
    Dst,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Opcode {
    NOp,
    AssertEq,
    Call,
    Ret,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum OpcodeExtension {
    Stone,
    Blake,
    BlakeFinalize,
    QM31Operation,
}

impl Instruction {
    pub fn size(&self) -> usize {
        match self.op1_addr {
            Op1Addr::Imm => 2,
            _ => 1,
        }
    }
}

// Returns True if the given instruction looks like a call instruction
pub(crate) fn is_call_instruction(encoded_instruction: &Felt252) -> bool {
    let encoded_u128_instruction = match encoded_instruction.to_u128() {
        Some(num) => num,
        None => return false,
    };
    let instruction = match decode_instruction(encoded_u128_instruction) {
        Ok(inst) => inst,
        Err(_) => return false,
    };
    instruction.res == Res::Op1
        && (instruction.pc_update == PcUpdate::Jump || instruction.pc_update == PcUpdate::JumpRel)
        && instruction.ap_update == ApUpdate::Add2
        && instruction.fp_update == FpUpdate::APPlus2
        && instruction.opcode == Opcode::Call
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn is_call_instruction_true() {
        let encoded_instruction = Felt252::from(1226245742482522112_i64);
        assert!(is_call_instruction(&encoded_instruction));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn is_call_instruction_false() {
        let encoded_instruction = Felt252::from(4612671187288031229_i64);
        assert!(!is_call_instruction(&encoded_instruction));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn is_call_instruction_invalid() {
        let encoded_instruction = Felt252::from(1u64 << 63);
        assert!(!is_call_instruction(&encoded_instruction));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn instruction_size() {
        let encoded_instruction = Felt252::from(1226245742482522112_i64);
        let instruction = decode_instruction(encoded_instruction.to_u128().unwrap()).unwrap();
        assert_eq!(instruction.size(), 2);
    }
}
