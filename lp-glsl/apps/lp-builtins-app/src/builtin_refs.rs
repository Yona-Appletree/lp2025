//! This file is AUTO-GENERATED. Do not edit manually.
//!
//! To regenerate this file, run:
//!     cargo run --bin lp-builtin-gen --manifest-path lp-glsl/apps/lp-builtin-gen/Cargo.toml
//!
//! Or use the build script:
//!     scripts/build-builtins.sh

use lp_builtins::builtins::fixed32::{
    __lp_fixed32_acos, __lp_fixed32_acosh, __lp_fixed32_add, __lp_fixed32_asin, __lp_fixed32_asinh,
    __lp_fixed32_atan, __lp_fixed32_atan2, __lp_fixed32_atanh, __lp_fixed32_cos, __lp_fixed32_cosh,
    __lp_fixed32_div, __lp_fixed32_exp, __lp_fixed32_exp2, __lp_fixed32_fma,
    __lp_fixed32_inversesqrt, __lp_fixed32_ldexp, __lp_fixed32_log, __lp_fixed32_log2,
    __lp_fixed32_mod, __lp_fixed32_mul, __lp_fixed32_pow, __lp_fixed32_round,
    __lp_fixed32_roundeven, __lp_fixed32_sin, __lp_fixed32_sinh, __lp_fixed32_sqrt,
    __lp_fixed32_sub, __lp_fixed32_tan, __lp_fixed32_tanh,
};
use lp_builtins::builtins::lpfx::hash::{__lpfx_hash_1, __lpfx_hash_2, __lpfx_hash_3};
use lp_builtins::builtins::lpfx::simplex::{
    simplex1_f32::__lpfx_simplex1_f32, simplex1_q32::__lpfx_simplex1_q32,
    simplex2_f32::__lpfx_simplex2_f32, simplex2_q32::__lpfx_simplex2_q32,
    simplex3_f32::__lpfx_simplex3_f32, simplex3_q32::__lpfx_simplex3_q32,
};

/// Reference all builtin functions to prevent dead code elimination.
///
/// This function ensures all builtin functions are included in the executable
/// by creating function pointers and reading them with volatile operations.
pub fn ensure_builtins_referenced() {
    unsafe {
        let _fixed32_acos_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_acos;
        let _fixed32_acosh_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_acosh;
        let _fixed32_add_fn: extern "C" fn(i32, i32) -> i32 = __lp_fixed32_add;
        let _fixed32_asin_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_asin;
        let _fixed32_asinh_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_asinh;
        let _fixed32_atan_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_atan;
        let _fixed32_atan2_fn: extern "C" fn(i32, i32) -> i32 = __lp_fixed32_atan2;
        let _fixed32_atanh_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_atanh;
        let _fixed32_cos_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_cos;
        let _fixed32_cosh_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_cosh;
        let _fixed32_div_fn: extern "C" fn(i32, i32) -> i32 = __lp_fixed32_div;
        let _fixed32_exp_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_exp;
        let _fixed32_exp2_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_exp2;
        let _fixed32_fma_fn: extern "C" fn(i32, i32, i32) -> i32 = __lp_fixed32_fma;
        let _fixed32_inversesqrt_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_inversesqrt;
        let _fixed32_ldexp_fn: extern "C" fn(i32, i32) -> i32 = __lp_fixed32_ldexp;
        let _fixed32_log_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_log;
        let _fixed32_log2_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_log2;
        let _fixed32_mod_fn: extern "C" fn(i32, i32) -> i32 = __lp_fixed32_mod;
        let _fixed32_mul_fn: extern "C" fn(i32, i32) -> i32 = __lp_fixed32_mul;
        let _fixed32_pow_fn: extern "C" fn(i32, i32) -> i32 = __lp_fixed32_pow;
        let _fixed32_round_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_round;
        let _fixed32_roundeven_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_roundeven;
        let _fixed32_sin_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_sin;
        let _fixed32_sinh_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_sinh;
        let _fixed32_sqrt_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_sqrt;
        let _fixed32_sub_fn: extern "C" fn(i32, i32) -> i32 = __lp_fixed32_sub;
        let _fixed32_tan_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_tan;
        let _fixed32_tanh_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_tanh;
        let __lpfx_hash_1_fn: extern "C" fn(u32, u32) -> u32 = __lpfx_hash_1;
        let __lpfx_hash_2_fn: extern "C" fn(u32, u32, u32) -> u32 = __lpfx_hash_2;
        let __lpfx_hash_3_fn: extern "C" fn(u32, u32, u32, u32) -> u32 = __lpfx_hash_3;
        let __lpfx_simplex1_f32_fn: extern "C" fn(f32, u32) -> f32 = __lpfx_simplex1_f32;
        let __lpfx_simplex1_q32_fn: extern "C" fn(i32, u32) -> i32 = __lpfx_simplex1_q32;
        let __lpfx_simplex2_f32_fn: extern "C" fn(f32, f32, u32) -> f32 = __lpfx_simplex2_f32;
        let __lpfx_simplex2_q32_fn: extern "C" fn(i32, i32, u32) -> i32 = __lpfx_simplex2_q32;
        let __lpfx_simplex3_f32_fn: extern "C" fn(f32, f32, f32, u32) -> f32 = __lpfx_simplex3_f32;
        let __lpfx_simplex3_q32_fn: extern "C" fn(i32, i32, i32, u32) -> i32 = __lpfx_simplex3_q32;

        // Force these to be included by using them in a way that can't be optimized away
        // We'll use volatile reads to prevent optimization
        let _ = core::ptr::read_volatile(&_fixed32_acos_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_acosh_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_add_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_asin_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_asinh_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_atan_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_atan2_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_atanh_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_cos_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_cosh_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_div_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_exp_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_exp2_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_fma_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_inversesqrt_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_ldexp_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_log_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_log2_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_mod_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_mul_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_pow_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_round_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_roundeven_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_sin_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_sinh_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_sqrt_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_sub_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_tan_fn as *const _);
        let _ = core::ptr::read_volatile(&_fixed32_tanh_fn as *const _);
        let _ = core::ptr::read_volatile(&__lpfx_hash_1_fn as *const _);
        let _ = core::ptr::read_volatile(&__lpfx_hash_2_fn as *const _);
        let _ = core::ptr::read_volatile(&__lpfx_hash_3_fn as *const _);
        let _ = core::ptr::read_volatile(&__lpfx_simplex1_f32_fn as *const _);
        let _ = core::ptr::read_volatile(&__lpfx_simplex1_q32_fn as *const _);
        let _ = core::ptr::read_volatile(&__lpfx_simplex2_f32_fn as *const _);
        let _ = core::ptr::read_volatile(&__lpfx_simplex2_q32_fn as *const _);
        let _ = core::ptr::read_volatile(&__lpfx_simplex3_f32_fn as *const _);
        let _ = core::ptr::read_volatile(&__lpfx_simplex3_q32_fn as *const _);
    }
}
