//! Math function conversion functions.

use crate::backend::builtins::registry::BuiltinId;
use crate::backend::transform::q32::converters::{
    extract_unary_operand, get_first_result, map_operand,
};
use crate::backend::transform::q32::types::FixedPointFormat;
use crate::error::GlslError;
use cranelift_codegen::ir::{Function, Inst, InstBuilder, Value, types};
use cranelift_frontend::FunctionBuilder;
use hashbrown::HashMap;

use alloc::format;

/// Map TestCase function name to BuiltinId and argument count.
///
/// Returns None if the function name is not a math function that should be converted.
/// Handles both standard C math function names (sinf, cosf) and intrinsic names (__lp_sin, __lp_cos).
/// Returns (BuiltinId, argument_count) where argument_count is 1 or 2.
///
/// This function is AUTO-GENERATED. Do not edit manually.
///
/// To regenerate this function, run:
///     cargo run --bin lp-builtin-gen --manifest-path lp-glsl/apps/lp-builtin-gen/Cargo.toml
///
/// Or use the build script:
///     scripts/build-builtins.sh
pub fn map_testcase_to_builtin(testcase_name: &str) -> Option<(BuiltinId, usize)> {
    match testcase_name {
        "lp_q32_acosf" | "__lp_q32_acos" | "acosf" => Some((BuiltinId::LpQ32Acos, 1)),
        "lp_q32_acoshf" | "__lp_q32_acosh" | "acoshf" => Some((BuiltinId::LpQ32Acosh, 1)),
        "lp_q32_addf" | "__lp_q32_add" | "addf" => Some((BuiltinId::LpQ32Add, 2)),
        "lp_q32_asinf" | "__lp_q32_asin" | "asinf" => Some((BuiltinId::LpQ32Asin, 1)),
        "lp_q32_asinhf" | "__lp_q32_asinh" | "asinhf" => Some((BuiltinId::LpQ32Asinh, 1)),
        "lp_q32_atanf" | "__lp_q32_atan" | "atanf" => Some((BuiltinId::LpQ32Atan, 1)),
        "lp_q32_atan2f" | "__lp_q32_atan2" | "atan2f" => Some((BuiltinId::LpQ32Atan2, 2)),
        "lp_q32_atanhf" | "__lp_q32_atanh" | "atanhf" => Some((BuiltinId::LpQ32Atanh, 1)),
        "lp_q32_cosf" | "__lp_q32_cos" | "cosf" => Some((BuiltinId::LpQ32Cos, 1)),
        "lp_q32_coshf" | "__lp_q32_cosh" | "coshf" => Some((BuiltinId::LpQ32Cosh, 1)),
        "lp_q32_divf" | "__lp_q32_div" | "divf" => Some((BuiltinId::LpQ32Div, 2)),
        "lp_q32_expf" | "__lp_q32_exp" | "expf" => Some((BuiltinId::LpQ32Exp, 1)),
        "lp_q32_exp2f" | "__lp_q32_exp2" | "exp2f" => Some((BuiltinId::LpQ32Exp2, 1)),
        "lp_q32_fmaf" | "__lp_q32_fma" | "fmaf" => Some((BuiltinId::LpQ32Fma, 3)),
        "lp_q32_inversesqrtf" | "__lp_q32_inversesqrt" | "inversesqrtf" => {
            Some((BuiltinId::LpQ32Inversesqrt, 1))
        }
        "lp_q32_ldexpf" | "__lp_q32_ldexp" | "ldexpf" => Some((BuiltinId::LpQ32Ldexp, 2)),
        "lp_q32_logf" | "__lp_q32_log" | "logf" => Some((BuiltinId::LpQ32Log, 1)),
        "lp_q32_log2f" | "__lp_q32_log2" | "log2f" => Some((BuiltinId::LpQ32Log2, 1)),
        "lp_q32_modf" | "__lp_q32_mod" | "fmodf" => Some((BuiltinId::LpQ32Mod, 2)),
        "lp_q32_mulf" | "__lp_q32_mul" | "mulf" => Some((BuiltinId::LpQ32Mul, 2)),
        "lp_q32_powf" | "__lp_q32_pow" | "powf" => Some((BuiltinId::LpQ32Pow, 2)),
        "lp_q32_roundf" | "__lp_q32_round" | "roundf" => Some((BuiltinId::LpQ32Round, 1)),
        "lp_q32_roundevenf" | "__lp_q32_roundeven" | "roundevenf" => {
            Some((BuiltinId::LpQ32Roundeven, 1))
        }
        "lp_q32_sinf" | "__lp_q32_sin" | "sinf" => Some((BuiltinId::LpQ32Sin, 1)),
        "lp_q32_sinhf" | "__lp_q32_sinh" | "sinhf" => Some((BuiltinId::LpQ32Sinh, 1)),
        "lp_q32_sqrtf" | "__lp_q32_sqrt" | "sqrtf" => Some((BuiltinId::LpQ32Sqrt, 1)),
        "lp_q32_subf" | "__lp_q32_sub" | "subf" => Some((BuiltinId::LpQ32Sub, 2)),
        "lp_q32_tanf" | "__lp_q32_tan" | "tanf" => Some((BuiltinId::LpQ32Tan, 1)),
        "lp_q32_tanhf" | "__lp_q32_tanh" | "tanhf" => Some((BuiltinId::LpQ32Tanh, 1)),
        "lpfx_hash_1f" | "__lp_lpfx_hash_1" => Some((BuiltinId::LpfxHash1, 2)),
        "lpfx_hash_2f" | "__lp_lpfx_hash_2" => Some((BuiltinId::LpfxHash2, 3)),
        "lpfx_hash_3f" | "__lp_lpfx_hash_3" => Some((BuiltinId::LpfxHash3, 4)),
        "lpfx_hsv2rgb_q32f" | "__lpfx_hsv2rgb_q32" => Some((BuiltinId::LpfxHsv2rgbQ32, 3)),
        "__lpfx_hsv2rgb" => Some((BuiltinId::LpfxHsv2rgbVec4Q32, 4)),
        "__lpfx_hue2rgb" => Some((BuiltinId::LpfxHue2rgbQ32, 1)),
        "lpfx_rgb2hsv_q32f" | "__lpfx_rgb2hsv_q32" => Some((BuiltinId::LpfxRgb2hsvQ32, 3)),
        "__lpfx_rgb2hsv" => Some((BuiltinId::LpfxRgb2hsvVec4Q32, 4)),
        "lpfx_saturate_q32f" | "__lpfx_saturate_q32" => Some((BuiltinId::LpfxSaturateQ32, 1)),
        "lpfx_saturate_vec3_q32f" | "__lpfx_saturate_vec3_q32" => {
            Some((BuiltinId::LpfxSaturateVec3Q32, 3))
        }
        "__lpfx_saturate" => Some((BuiltinId::LpfxSaturateVec4Q32, 4)),
        "__lpfx_snoise1" => Some((BuiltinId::LpfxSnoise1Q32, 2)),
        "__lpfx_snoise2" => Some((BuiltinId::LpfxSnoise2Q32, 3)),
        "__lpfx_snoise3" => Some((BuiltinId::LpfxSnoise3Q32, 4)),
        "__lpfx_worley2" => Some((BuiltinId::LpfxWorley2Q32, 3)),
        "__lpfx_worley2_value" => Some((BuiltinId::LpfxWorley2ValueQ32, 3)),
        "__lpfx_worley3" => Some((BuiltinId::LpfxWorley3Q32, 4)),
        "__lpfx_worley3_value" => Some((BuiltinId::LpfxWorley3ValueQ32, 4)),
        _ => None,
    }
}

/// Convert Ceil instruction.
pub(crate) fn convert_ceil(
    old_func: &Function,
    old_inst: Inst,
    builder: &mut FunctionBuilder,
    value_map: &mut HashMap<Value, Value>,
    format: FixedPointFormat,
) -> Result<(), GlslError> {
    let arg = extract_unary_operand(old_func, old_inst)?;
    let mapped_arg = map_operand(old_func, value_map, arg)?;
    let target_type = format.cranelift_type();
    let shift_amount = format.shift_amount();

    // Ceil: round up to nearest integer
    // In fixed-point: (value + (1 << shift) - 1) >> shift, then << shift
    let mask = (1i64 << shift_amount) - 1;
    let mask_const = builder.ins().iconst(target_type, mask);
    let added = builder.ins().iadd(mapped_arg, mask_const);
    let shift_const = builder.ins().iconst(target_type, shift_amount);
    let rounded = builder.ins().sshr(added, shift_const);
    let new_result = builder.ins().ishl(rounded, shift_const);

    let old_result = get_first_result(old_func, old_inst);
    value_map.insert(old_result, new_result);

    Ok(())
}

/// Convert Floor instruction.
pub(crate) fn convert_floor(
    old_func: &Function,
    old_inst: Inst,
    builder: &mut FunctionBuilder,
    value_map: &mut HashMap<Value, Value>,
    format: FixedPointFormat,
) -> Result<(), GlslError> {
    let arg = extract_unary_operand(old_func, old_inst)?;
    let mapped_arg = map_operand(old_func, value_map, arg)?;
    let target_type = format.cranelift_type();
    let shift_amount = format.shift_amount();

    // Floor: round down to nearest integer
    // In fixed-point: value >> shift, then << shift
    let shift_const = builder.ins().iconst(target_type, shift_amount);
    let rounded = builder.ins().sshr(mapped_arg, shift_const);
    let new_result = builder.ins().ishl(rounded, shift_const);

    let old_result = get_first_result(old_func, old_inst);
    value_map.insert(old_result, new_result);

    Ok(())
}

/// Convert Trunc instruction.
pub(crate) fn convert_trunc(
    old_func: &Function,
    old_inst: Inst,
    builder: &mut FunctionBuilder,
    value_map: &mut HashMap<Value, Value>,
    format: FixedPointFormat,
) -> Result<(), GlslError> {
    // Trunc is the same as floor for positive numbers, but rounds toward zero
    // For fixed-point, we can use the same approach as floor
    convert_floor(old_func, old_inst, builder, value_map, format)
}

/// Convert Nearest instruction (round to nearest).
pub(crate) fn convert_nearest(
    old_func: &Function,
    old_inst: Inst,
    builder: &mut FunctionBuilder,
    value_map: &mut HashMap<Value, Value>,
    format: FixedPointFormat,
) -> Result<(), GlslError> {
    let arg = extract_unary_operand(old_func, old_inst)?;
    let mapped_arg = map_operand(old_func, value_map, arg)?;
    let target_type = format.cranelift_type();
    let shift_amount = format.shift_amount();

    // Nearest: round to nearest integer
    // In fixed-point: (value + (1 << (shift - 1))) >> shift, then << shift
    let half = 1i64 << (shift_amount - 1);
    let half_const = builder.ins().iconst(target_type, half);
    let added = builder.ins().iadd(mapped_arg, half_const);
    let shift_const = builder.ins().iconst(target_type, shift_amount);
    let rounded = builder.ins().sshr(added, shift_const);
    let new_result = builder.ins().ishl(rounded, shift_const);

    let old_result = get_first_result(old_func, old_inst);
    value_map.insert(old_result, new_result);

    Ok(())
}

/// Convert Sqrt by calling the linked __lp_q32_sqrt function.
pub(crate) fn convert_sqrt(
    old_func: &Function,
    old_inst: Inst,
    builder: &mut FunctionBuilder,
    value_map: &mut HashMap<Value, Value>,
    _format: FixedPointFormat,
    func_id_map: &HashMap<alloc::string::String, cranelift_module::FuncId>,
) -> Result<(), GlslError> {
    use cranelift_codegen::ir::{AbiParam, ExtFuncData, ExternalName, Signature, UserExternalName};
    use cranelift_codegen::isa::CallConv;

    let arg = extract_unary_operand(old_func, old_inst)?;
    let mapped_arg = map_operand(old_func, value_map, arg)?;

    // Get FuncId for __lp_q32_sqrt from func_id_map
    let builtin_name = "__lp_q32_sqrt";
    let func_id = func_id_map.get(builtin_name).ok_or_else(|| {
        GlslError::new(
            crate::error::ErrorCode::E0400,
            format!("Builtin function '{builtin_name}' not found in func_id_map"),
        )
    })?;

    // Create signature for __lp_q32_sqrt: (i32) -> i32
    let mut sig = Signature::new(CallConv::SystemV);
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
    let sqrt_func_ref = builder.func.import_function(ext_func);

    // Call __lp_q32_sqrt with the mapped argument
    let call_result = builder.ins().call(sqrt_func_ref, &[mapped_arg]);
    let result = builder.inst_results(call_result)[0];

    let old_result = get_first_result(old_func, old_inst);
    value_map.insert(old_result, result);

    Ok(())
}

#[cfg(test)]
#[cfg(feature = "std")]
mod tests {
    use crate::backend::transform::q32::q32_test_util;

    /// Test sqrt: square root
    ///
    /// NOTE: This test is currently ignored because sqrt uses i64 division
    /// which is not supported on riscv32. We'll need to implement an alternative
    /// algorithm that doesn't require i64 division.
    #[test]
    #[cfg(feature = "emulator")]
    #[ignore]
    fn test_q32_sqrt() {
        let clif = r#"
function %main() -> f32 system_v {
block0:
    v0 = f32const 0x1.0p2
    v1 = sqrt v0
    return v1
}
"#;
        // Result should be 2.0 (sqrt of 4.0)
        // Note: Newton-Raphson may have some precision error, so we allow a small tolerance
        q32_test_util::run_q32_test(clif, 2.0);
    }

    /// Test sqrt: square root of 9.0
    ///
    /// NOTE: This test is currently ignored because sqrt uses i64 division
    /// which is not supported on riscv32.
    #[test]
    #[cfg(feature = "emulator")]
    #[ignore]
    fn test_q32_sqrt_9() {
        let clif = r#"
function %main() -> f32 system_v {
block0:
    v0 = f32const 0x1.2p3
    v1 = sqrt v0
    return v1
}
"#;
        // Result should be 3.0 (sqrt of 9.0)
        q32_test_util::run_q32_test(clif, 3.0);
    }

    /// Test sqrt: square root of zero
    ///
    /// NOTE: This test is currently ignored because sqrt uses i64 division
    /// which is not supported on riscv32.
    #[test]
    #[cfg(feature = "emulator")]
    #[ignore]
    fn test_q32_sqrt_zero() {
        let clif = r#"
function %main() -> f32 system_v {
block0:
    v0 = f32const 0x0.0p0
    v1 = sqrt v0
    return v1
}
"#;
        // Result should be 0.0
        q32_test_util::run_q32_test(clif, 0.0);
    }

    // Unit tests for map_testcase_to_builtin
    use super::map_testcase_to_builtin;
    use crate::backend::builtins::registry::BuiltinId;

    #[test]
    fn test_map_testcase_to_builtin_simplex() {
        // Test simplex function mappings
        assert_eq!(
            map_testcase_to_builtin("__lpfx_snoise1"),
            Some((BuiltinId::LpfxSnoise1Q32, 2))
        );
        assert_eq!(
            map_testcase_to_builtin("__lpfx_snoise2"),
            Some((BuiltinId::LpfxSnoise2Q32, 3))
        );
        assert_eq!(
            map_testcase_to_builtin("__lpfx_snoise3"),
            Some((BuiltinId::LpfxSnoise3Q32, 4))
        );
    }

    #[test]
    fn test_map_testcase_to_builtin_hash() {
        // Test hash function mappings
        assert_eq!(
            map_testcase_to_builtin("lpfx_hash_1f"),
            Some((BuiltinId::LpfxHash1, 2))
        );
        assert_eq!(
            map_testcase_to_builtin("__lp_lpfx_hash_1"),
            Some((BuiltinId::LpfxHash1, 2))
        );
        assert_eq!(
            map_testcase_to_builtin("lpfx_hash_2f"),
            Some((BuiltinId::LpfxHash2, 3))
        );
        assert_eq!(
            map_testcase_to_builtin("__lp_lpfx_hash_2"),
            Some((BuiltinId::LpfxHash2, 3))
        );
        assert_eq!(
            map_testcase_to_builtin("lpfx_hash_3f"),
            Some((BuiltinId::LpfxHash3, 4))
        );
        assert_eq!(
            map_testcase_to_builtin("__lp_lpfx_hash_3"),
            Some((BuiltinId::LpfxHash3, 4))
        );
    }

    #[test]
    fn test_map_testcase_to_builtin_standard_math() {
        // Test a few standard math function mappings
        assert_eq!(
            map_testcase_to_builtin("lp_q32_sinf"),
            Some((BuiltinId::LpQ32Sin, 1))
        );
        assert_eq!(
            map_testcase_to_builtin("__lp_q32_sin"),
            Some((BuiltinId::LpQ32Sin, 1))
        );
        assert_eq!(
            map_testcase_to_builtin("lp_q32_addf"),
            Some((BuiltinId::LpQ32Add, 2))
        );
        assert_eq!(
            map_testcase_to_builtin("__lp_q32_add"),
            Some((BuiltinId::LpQ32Add, 2))
        );
        assert_eq!(
            map_testcase_to_builtin("lp_q32_fmaf"),
            Some((BuiltinId::LpQ32Fma, 3))
        );
        assert_eq!(
            map_testcase_to_builtin("__lp_q32_fma"),
            Some((BuiltinId::LpQ32Fma, 3))
        );
    }

    #[test]
    fn test_map_testcase_to_builtin_unknown() {
        // Test that unknown functions return None
        assert_eq!(map_testcase_to_builtin("unknown_function"), None);
        assert_eq!(map_testcase_to_builtin("__lp_unknown"), None);
        assert_eq!(map_testcase_to_builtin(""), None);
    }
}
