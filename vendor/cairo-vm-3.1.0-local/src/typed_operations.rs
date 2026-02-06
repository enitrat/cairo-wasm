use crate::math_utils::{
    qm31_packed_reduced_add, qm31_packed_reduced_div, qm31_packed_reduced_mul,
    qm31_packed_reduced_sub,
};
use crate::stdlib::prelude::*;
use crate::types::relocatable::MaybeRelocatable;
use crate::types::{errors::math_errors::MathError, instruction::OpcodeExtension};
use crate::vm::errors::vm_errors::VirtualMachineError;
use crate::Felt252;

/// Adds two MaybeRelocatable values according to the specified OpcodeExtension and returns the
/// result as a MaybeRelocatable value.
/// If the OpcodeExtension is Stone it adds them as MaybeRelocatable::add does.
/// If the OpcodeExtension is QM31Operation it requires them both to be Int and it adds them
/// as packed reduced QM31 elements.
pub fn typed_add(
    x: &MaybeRelocatable,
    y: &MaybeRelocatable,
    opcode_extension: OpcodeExtension,
) -> Result<MaybeRelocatable, VirtualMachineError> {
    match opcode_extension {
        OpcodeExtension::Stone => Ok(x.add(y)?),
        OpcodeExtension::QM31Operation => {
            if let (MaybeRelocatable::Int(num_x), MaybeRelocatable::Int(num_y)) = (x, y) {
                Ok(MaybeRelocatable::Int(qm31_packed_reduced_add(
                    *num_x, *num_y,
                )?))
            } else {
                Err(VirtualMachineError::Math(MathError::RelocatableQM31Add(
                    Box::new((x.clone(), y.clone())),
                )))
            }
        }
        _ => Err(VirtualMachineError::InvalidTypedOperationOpcodeExtension(
            "typed_add".to_owned().into_boxed_str(),
        )),
    }
}

/// Substracts two MaybeRelocatable values according to the specified OpcodeExtension and returns
/// the result as a MaybeRelocatable value.
/// If the OpcodeExtension is Stone it subtracts them as MaybeRelocatable::sub does.
/// If the OpcodeExtension is QM31Operation it requires them both to be Int and it subtracts
/// them as packed reduced QM31 elements.
pub fn typed_sub(
    x: &MaybeRelocatable,
    y: &MaybeRelocatable,
    opcode_extension: OpcodeExtension,
) -> Result<MaybeRelocatable, VirtualMachineError> {
    match opcode_extension {
        OpcodeExtension::Stone => Ok(x.sub(y)?),
        OpcodeExtension::QM31Operation => {
            if let (MaybeRelocatable::Int(num_x), MaybeRelocatable::Int(num_y)) = (x, y) {
                Ok(MaybeRelocatable::Int(qm31_packed_reduced_sub(
                    *num_x, *num_y,
                )?))
            } else {
                Err(VirtualMachineError::Math(MathError::RelocatableQM31Sub(
                    Box::new((x.clone(), y.clone())),
                )))
            }
        }
        _ => Err(VirtualMachineError::InvalidTypedOperationOpcodeExtension(
            "typed_sub".to_owned().into_boxed_str(),
        )),
    }
}

/// Multiplies two MaybeRelocatable values according to the specified OpcodeExtension and returns
/// the result as a MaybeRelocatable value.
/// Requires both operands to be Int.
/// If the OpcodeExtension is Stone it multiplies them as Felts.
/// If the OpcodeExtension is QM31Operation it multiplies them as packed reduced QM31 elements.
pub fn typed_mul(
    x: &MaybeRelocatable,
    y: &MaybeRelocatable,
    opcode_extension: OpcodeExtension,
) -> Result<MaybeRelocatable, VirtualMachineError> {
    if let (MaybeRelocatable::Int(num_x), MaybeRelocatable::Int(num_y)) = (x, y) {
        match opcode_extension {
            OpcodeExtension::Stone => Ok(MaybeRelocatable::Int(num_x * num_y)),
            OpcodeExtension::QM31Operation => Ok(MaybeRelocatable::Int(qm31_packed_reduced_mul(
                *num_x, *num_y,
            )?)),
            _ => Err(VirtualMachineError::InvalidTypedOperationOpcodeExtension(
                "typed_mul".to_owned().into_boxed_str(),
            )),
        }
    } else {
        Err(VirtualMachineError::ComputeResRelocatableMul(Box::new((
            x.clone(),
            y.clone(),
        ))))
    }
}

/// Divides two Felt252 values according to the specified OpcodeExtension and returns the result
/// as a Felt252 value.
/// If the OpcodeExtension is Stone it divides them as Felts.
/// If the OpcodeExtension is QM31Operation it divides them as packed reduced QM31 elements.
pub fn typed_div(
    x: &Felt252,
    y: &Felt252,
    opcode_extension: OpcodeExtension,
) -> Result<Felt252, VirtualMachineError> {
    match opcode_extension {
        OpcodeExtension::Stone => {
            Ok(x.field_div(&y.try_into().map_err(|_| MathError::DividedByZero)?))
        }
        OpcodeExtension::QM31Operation => Ok(qm31_packed_reduced_div(*x, *y)?),
        _ => Err(VirtualMachineError::InvalidTypedOperationOpcodeExtension(
            "typed_div".to_owned().into_boxed_str(),
        )),
    }
}
#[cfg(test)]
mod decoder_test {
    use super::*;
    use assert_matches::assert_matches;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn typed_add_blake() {
        let a = &MaybeRelocatable::from(5);
        let b = &MaybeRelocatable::from(6);
        let error = typed_add(a, b, OpcodeExtension::Blake);
        assert_matches!(
            error,
            Err(VirtualMachineError::InvalidTypedOperationOpcodeExtension(ref message)) if message.as_ref() == "typed_add"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn typed_sub_blake() {
        let a = &MaybeRelocatable::from(7);
        let b = &MaybeRelocatable::from(3);
        let error = typed_sub(a, b, OpcodeExtension::Blake);
        assert_matches!(
            error,
            Err(VirtualMachineError::InvalidTypedOperationOpcodeExtension(ref message)) if message.as_ref() == "typed_sub"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn relocatable_typed_sub_q31_operation() {
        let a = &MaybeRelocatable::from((6, 8));
        let b = &MaybeRelocatable::from(2);
        let error = typed_sub(a, b, OpcodeExtension::QM31Operation);
        assert_matches!(
            error,
            Err(VirtualMachineError::Math(MathError::RelocatableQM31Sub(bx))) if *bx ==
                (MaybeRelocatable::from((6, 8)), MaybeRelocatable::from(2))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn typed_mul_blake_finalize() {
        let a = &MaybeRelocatable::from(4);
        let b = &MaybeRelocatable::from(9);
        let error = typed_mul(a, b, OpcodeExtension::BlakeFinalize);
        assert_matches!(
            error,
            Err(VirtualMachineError::InvalidTypedOperationOpcodeExtension(ref message)) if message.as_ref() == "typed_mul"
        );
    }
}
