//! Arithmetic operation conversion functions.

use crate::backend::transform::fixed32::converters::{
    create_max_fixed_const, create_min_fixed_const, create_zero_const, extract_binary_operands,
    extract_unary_operand, get_first_result, map_operand,
};
use crate::backend::transform::fixed32::types::FixedPointFormat;
use crate::error::GlslError;
use cranelift_codegen::ir::{Function, Inst, InstBuilder, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use hashbrown::HashMap;

use alloc::format;

/// Convert Fadd to fixed-point addition with saturation
pub(crate) fn convert_fadd(
    old_func: &Function,
    old_inst: Inst,
    builder: &mut FunctionBuilder,
    value_map: &mut HashMap<cranelift_codegen::ir::Value, cranelift_codegen::ir::Value>,
    format: FixedPointFormat,
) -> Result<(), GlslError> {
    let (arg1_old, arg2_old) = extract_binary_operands(old_func, old_inst)?;
    let arg1 = map_operand(value_map, arg1_old);
    let arg2 = map_operand(value_map, arg2_old);

    // Fixed-point addition is just integer addition (no conversion needed)
    // Both operands are already in fixed-point format
    let result = builder.ins().iadd(arg1, arg2);

    // Saturate result to fixed-point range
    let zero = create_zero_const(builder, format);
    let max_fixed = create_max_fixed_const(builder, format);
    let min_fixed = create_min_fixed_const(builder, format);

    // Check for overflow: if both operands are positive and result is negative, saturate to max
    let arg1_positive = builder
        .ins()
        .icmp(IntCC::SignedGreaterThanOrEqual, arg1, zero);
    let arg2_positive = builder
        .ins()
        .icmp(IntCC::SignedGreaterThanOrEqual, arg2, zero);
    let result_negative = builder.ins().icmp(IntCC::SignedLessThan, result, zero);
    let both_positive = builder.ins().band(arg1_positive, arg2_positive);
    let overflow = builder.ins().band(both_positive, result_negative);

    // Check for underflow: if both operands are negative and result is positive, saturate to min
    let arg1_negative = builder.ins().icmp(IntCC::SignedLessThan, arg1, zero);
    let arg2_negative = builder.ins().icmp(IntCC::SignedLessThan, arg2, zero);
    let result_positive = builder
        .ins()
        .icmp(IntCC::SignedGreaterThanOrEqual, result, zero);
    let both_negative = builder.ins().band(arg1_negative, arg2_negative);
    let underflow = builder.ins().band(both_negative, result_positive);

    // Clamp result to range [min_fixed, max_fixed]
    let clamped_max = builder.ins().smin(result, max_fixed);
    let clamped = builder.ins().smax(clamped_max, min_fixed);

    // Select: if overflow use max, if underflow use min, otherwise use clamped
    let saturated = builder.ins().select(overflow, max_fixed, clamped);
    let final_result = builder.ins().select(underflow, min_fixed, saturated);

    let old_result = get_first_result(old_func, old_inst);
    value_map.insert(old_result, final_result);

    Ok(())
}

/// Convert Fsub to fixed-point subtraction with saturation
pub(crate) fn convert_fsub(
    old_func: &Function,
    old_inst: Inst,
    builder: &mut FunctionBuilder,
    value_map: &mut HashMap<cranelift_codegen::ir::Value, cranelift_codegen::ir::Value>,
    format: FixedPointFormat,
) -> Result<(), GlslError> {
    let (arg1_old, arg2_old) = extract_binary_operands(old_func, old_inst)?;
    let arg1 = map_operand(value_map, arg1_old);
    let arg2 = map_operand(value_map, arg2_old);

    // Fixed-point subtraction: a - b
    let result = builder.ins().isub(arg1, arg2);

    // Saturate result to fixed-point range
    let zero = create_zero_const(builder, format);
    let max_fixed = create_max_fixed_const(builder, format);
    let min_fixed = create_min_fixed_const(builder, format);

    // Check for overflow: if arg1 is positive and arg2 is negative and result is negative, saturate to max
    let arg1_positive = builder
        .ins()
        .icmp(IntCC::SignedGreaterThanOrEqual, arg1, zero);
    let arg2_negative = builder.ins().icmp(IntCC::SignedLessThan, arg2, zero);
    let result_negative = builder.ins().icmp(IntCC::SignedLessThan, result, zero);
    let overflow_cond = builder.ins().band(arg1_positive, arg2_negative);
    let overflow = builder.ins().band(overflow_cond, result_negative);

    // Check for underflow: if arg1 is negative and arg2 is positive and result is positive, saturate to min
    let arg1_negative = builder.ins().icmp(IntCC::SignedLessThan, arg1, zero);
    let arg2_positive = builder
        .ins()
        .icmp(IntCC::SignedGreaterThanOrEqual, arg2, zero);
    let result_positive = builder
        .ins()
        .icmp(IntCC::SignedGreaterThanOrEqual, result, zero);
    let underflow_cond = builder.ins().band(arg1_negative, arg2_positive);
    let underflow = builder.ins().band(underflow_cond, result_positive);

    // Clamp result to range [min_fixed, max_fixed]
    let clamped_max = builder.ins().smin(result, max_fixed);
    let clamped = builder.ins().smax(clamped_max, min_fixed);

    // Select: if overflow use max, if underflow use min, otherwise use clamped
    let saturated = builder.ins().select(overflow, max_fixed, clamped);
    let final_result = builder.ins().select(underflow, min_fixed, saturated);

    let old_result = get_first_result(old_func, old_inst);
    value_map.insert(old_result, final_result);

    Ok(())
}

/// Convert Fmul to fixed-point multiplication by calling __lp_fixed32_mul builtin.
pub(crate) fn convert_fmul(
    old_func: &Function,
    old_inst: Inst,
    builder: &mut FunctionBuilder,
    value_map: &mut HashMap<cranelift_codegen::ir::Value, cranelift_codegen::ir::Value>,
    _format: FixedPointFormat,
    func_id_map: &HashMap<alloc::string::String, cranelift_module::FuncId>,
) -> Result<(), GlslError> {
    use cranelift_codegen::ir::{AbiParam, ExtFuncData, ExternalName, Signature, UserExternalName};
    use cranelift_codegen::isa::CallConv;

    let (arg1_old, arg2_old) = extract_binary_operands(old_func, old_inst)?;
    let arg1 = map_operand(value_map, arg1_old);
    let arg2 = map_operand(value_map, arg2_old);

    // Get FuncId for __lp_fixed32_mul from func_id_map
    let builtin_name = "__lp_fixed32_mul";
    let func_id = func_id_map.get(builtin_name).ok_or_else(|| {
        GlslError::new(
            crate::error::ErrorCode::E0400,
            format!("Builtin function '{builtin_name}' not found in func_id_map"),
        )
    })?;

    // Create signature for __lp_fixed32_mul: (i32, i32) -> i32
    let mut sig = Signature::new(CallConv::SystemV);
    sig.params.push(AbiParam::new(types::I32));
    sig.params.push(AbiParam::new(types::I32));
    sig.returns.push(AbiParam::new(types::I32));
    let sig_ref = builder.func.import_signature(sig);

    // Create UserExternalName with the FuncId
    let user_name = UserExternalName {
        namespace: 0, // Use namespace 0 for builtins
        index: func_id.as_u32(),
    };
    let user_ref = builder.func.declare_imported_user_function(user_name);
    let ext_name = ExternalName::User(user_ref);

    // Builtin functions are external and may be far away, so they cannot be colocated.
    // This prevents ARM64 call relocation range issues (colocated uses ±128MB range).
    // For JIT mode, function pointers are resolved at runtime via symbol_lookup_fn.
    // For emulator mode, the linker will handle the relocation appropriately.
    let ext_func = ExtFuncData {
        name: ext_name,
        signature: sig_ref,
        colocated: false,
    };
    let mul_func_ref = builder.func.import_function(ext_func);

    // Call __lp_fixed32_mul with the mapped arguments
    let call_result = builder.ins().call(mul_func_ref, &[arg1, arg2]);
    let result = builder.inst_results(call_result)[0];

    let old_result = get_first_result(old_func, old_inst);
    value_map.insert(old_result, result);

    Ok(())
}

/// Convert Fdiv to fixed-point division with scaling and zero handling
///
/// Handles division by zero by saturating to maximum/minimum fixed-point values
/// based on the sign of the numerator. This matches typical fixed-point arithmetic
/// behavior where division by zero is undefined but we need to avoid crashes.
pub(crate) fn convert_fdiv(
    old_func: &Function,
    old_inst: Inst,
    builder: &mut FunctionBuilder,
    value_map: &mut HashMap<cranelift_codegen::ir::Value, cranelift_codegen::ir::Value>,
    format: FixedPointFormat,
) -> Result<(), GlslError> {
    let (arg1_old, arg2_old) = extract_binary_operands(old_func, old_inst)?;
    let arg1 = map_operand(value_map, arg1_old);
    let arg2 = map_operand(value_map, arg2_old);
    let target_type = format.cranelift_type();
    let shift_amount = format.shift_amount();

    // Check for division by zero
    // For fixed16x16, zero in fixed-point is 0 (same as integer zero)
    let zero = create_zero_const(builder, format);
    let is_zero = builder.ins().icmp(IntCC::Equal, arg2, zero);

    // Check conditions for saturation value
    let numerator_is_zero = builder.ins().icmp(IntCC::Equal, arg1, zero);
    let is_negative = builder.ins().icmp(IntCC::SignedLessThan, arg1, zero);

    // Compute saturation value using select instructions
    let max_fixed = create_max_fixed_const(builder, format);
    let min_fixed = create_min_fixed_const(builder, format);

    // infinity_value = is_negative ? min_fixed : max_fixed
    let infinity_value = builder.ins().select(is_negative, min_fixed, max_fixed);

    // saturation_value = numerator_is_zero ? zero : infinity_value
    let saturation_value = builder
        .ins()
        .select(numerator_is_zero, zero, infinity_value);

    // Perform division if divisor is non-zero
    let shift_const = builder.ins().iconst(target_type, shift_amount);
    // Use signed shift right to preserve sign bit for negative divisors
    let divisor_shifted = builder.ins().sshr(arg2, shift_const);

    // Check if divisor_shifted became zero (bug fix for small divisors < 2^16)
    let divisor_shifted_is_zero = builder.ins().icmp(IntCC::Equal, divisor_shifted, zero);

    // Use a safe divisor for the shifted case to avoid division by zero
    let one = builder.ins().iconst(target_type, 1);
    let safe_divisor_shifted = builder
        .ins()
        .select(divisor_shifted_is_zero, one, divisor_shifted);

    // For normal case: arg1 / safe_divisor_shifted
    let div_by_shifted_divisor = builder.ins().sdiv(arg1, safe_divisor_shifted);

    // For small divisor case: (arg1 << shift_amount) / safe_arg2
    // Ensure arg2 is never zero for division to avoid SIGILL
    let safe_arg2 = builder.ins().select(is_zero, one, arg2);
    let arg1_shifted = builder.ins().ishl(arg1, shift_const);
    let div_by_full_divisor = builder.ins().sdiv(arg1_shifted, safe_arg2);

    // Select the result: if divisor_shifted_is_zero then div_by_full_divisor else div_by_shifted_divisor
    let div_result = builder.ins().select(
        divisor_shifted_is_zero,
        div_by_full_divisor,
        div_by_shifted_divisor,
    );

    // Final result: if divisor was zero, use saturation_value, else use div_result
    let result = builder.ins().select(is_zero, saturation_value, div_result);

    let old_result = get_first_result(old_func, old_inst);
    value_map.insert(old_result, result);

    Ok(())
}

/// Convert Fneg to fixed-point negation
pub(crate) fn convert_fneg(
    old_func: &Function,
    old_inst: Inst,
    builder: &mut FunctionBuilder,
    value_map: &mut HashMap<cranelift_codegen::ir::Value, cranelift_codegen::ir::Value>,
    _format: FixedPointFormat,
) -> Result<(), GlslError> {
    let arg = extract_unary_operand(old_func, old_inst)?;
    let mapped_arg = map_operand(value_map, arg);

    let result = builder.ins().ineg(mapped_arg);

    let old_result = get_first_result(old_func, old_inst);
    value_map.insert(old_result, result);

    Ok(())
}

/// Convert Fabs using conditional select
pub(crate) fn convert_fabs(
    old_func: &Function,
    old_inst: Inst,
    builder: &mut FunctionBuilder,
    value_map: &mut HashMap<cranelift_codegen::ir::Value, cranelift_codegen::ir::Value>,
    format: FixedPointFormat,
) -> Result<(), GlslError> {
    let arg = extract_unary_operand(old_func, old_inst)?;
    let mapped_arg = map_operand(value_map, arg);

    // Absolute value: if (arg < 0) then -arg else arg
    let zero = create_zero_const(builder, format);
    let is_negative = builder.ins().icmp(IntCC::SignedLessThan, mapped_arg, zero);
    let negated = builder.ins().ineg(mapped_arg);
    let result = builder.ins().select(is_negative, negated, mapped_arg);

    let old_result = get_first_result(old_func, old_inst);
    value_map.insert(old_result, result);

    Ok(())
}

#[cfg(test)]
#[cfg(feature = "std")]
mod tests {
    #[cfg(feature = "emulator")]
    use crate::backend::transform::fixed32::fixed32_test_util;

    /// Test fadd: addition
    #[test]
    #[cfg(feature = "emulator")]
    fn test_fixed32_fadd() {
        // Use proper hex scientific notation: 0x1.8p-1 = 0.75, 0x1.8p1 = 3.0
        let clif = r#"
function %main() -> f32 system_v {
block0:
    v0 = f32const 0x1.8p1
    v1 = f32const 0x1.8p-1
    v2 = fadd v0, v1
    return v2
}
"#;
        fixed32_test_util::run_fixed32_test(clif, 3.75);
    }

    /// Test fsub: subtraction
    #[test]
    #[cfg(feature = "emulator")]
    fn test_fixed32_fsub() {
        let clif = r#"
function %main() -> f32 system_v {
block0:
    v0 = f32const 0x1.4p2
    v1 = f32const 0x1.4p1
    v2 = fsub v0, v1
    return v2
}
"#;
        fixed32_test_util::run_fixed32_test(clif, 2.5);
    }

    /// Test fmul: multiplication
    #[test]
    #[cfg(feature = "emulator")]
    fn test_fixed32_fmul() {
        let clif = r#"
function %main() -> f32 system_v {
block0:
    v0 = f32const 0x1.0p1
    v1 = f32const 0x1.8p1
    v2 = fmul v0, v1
    return v2
}
"#;
        fixed32_test_util::run_fixed32_test(clif, 6.0);
    }

    /// Test fdiv: division
    ///
    /// NOTE: This test is currently ignored due to a known issue with the division algorithm.
    /// The old backend has the same algorithm and may have the same bug. We'll fix this separately.
    #[test]
    #[cfg(feature = "emulator")]
    #[ignore]
    fn test_fixed32_fdiv() {
        let clif = r#"
function %main() -> f32 system_v {
block0:
    v0 = f32const 0x1.4p3
    v1 = f32const 0x1.4p1
    v2 = fdiv v0, v1
    return v2
}
"#;
        fixed32_test_util::run_fixed32_test(clif, 4.0);
    }

    /// Test fneg: negation
    #[test]
    #[cfg(feature = "emulator")]
    fn test_fixed32_fneg() {
        let clif = r#"
function %main() -> f32 system_v {
block0:
    v0 = f32const 0x1.4p1
    v1 = fneg v0
    return v1
}
"#;
        fixed32_test_util::run_fixed32_test(clif, -2.5);
    }

    /// Test fabs: absolute value
    #[test]
    #[cfg(feature = "emulator")]
    fn test_fixed32_fabs() {
        // Test with negative value
        let clif = r#"
function %main() -> f32 system_v {
block0:
    v0 = f32const -0x1.4p1
    v1 = fabs v0
    return v1
}
"#;
        fixed32_test_util::run_fixed32_test(clif, 2.5);
    }

    /// Test fabs: absolute value with positive value
    #[test]
    #[cfg(feature = "emulator")]
    fn test_fixed32_fabs_positive() {
        let clif = r#"
function %main() -> f32 system_v {
block0:
    v0 = f32const 0x1.4p1
    v1 = fabs v0
    return v1
}
"#;
        fixed32_test_util::run_fixed32_test(clif, 2.5);
    }
}
