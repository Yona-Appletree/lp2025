//! This file is AUTO-GENERATED. Do not edit manually.
//!
//! To regenerate this file, run:
//!     cargo run --bin lp-builtin-gen --manifest-path lp-glsl/apps/lp-builtin-gen/Cargo.toml
//!
//! Or use the build script:
//!     scripts/build-builtins.sh

use lp_builtins::builtins::fixed32::{
    __lp_fixed32_acos, __lp_fixed32_acosh, __lp_fixed32_asin, __lp_fixed32_asinh,
    __lp_fixed32_atan, __lp_fixed32_atan2, __lp_fixed32_atanh, __lp_fixed32_cos, __lp_fixed32_cosh,
    __lp_fixed32_div, __lp_fixed32_exp, __lp_fixed32_exp2, __lp_fixed32_fma,
    __lp_fixed32_inversesqrt, __lp_fixed32_ldexp, __lp_fixed32_log, __lp_fixed32_log2,
    __lp_fixed32_mod, __lp_fixed32_mul, __lp_fixed32_pow, __lp_fixed32_round,
    __lp_fixed32_roundeven, __lp_fixed32_sin, __lp_fixed32_sinh, __lp_fixed32_sqrt,
    __lp_fixed32_tan, __lp_fixed32_tanh,
};

/// Reference all builtin functions to prevent dead code elimination.
///
/// This function ensures all builtin functions are included in the executable
/// by creating function pointers and reading them with volatile operations.
pub fn ensure_builtins_referenced() {
    unsafe {
        let _acos_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_acos;
        let _acosh_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_acosh;
        let _asin_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_asin;
        let _asinh_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_asinh;
        let _atan_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_atan;
        let _atan2_fn: extern "C" fn(i32, i32) -> i32 = __lp_fixed32_atan2;
        let _atanh_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_atanh;
        let _cos_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_cos;
        let _cosh_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_cosh;
        let _div_fn: extern "C" fn(i32, i32) -> i32 = __lp_fixed32_div;
        let _exp_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_exp;
        let _exp2_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_exp2;
        let _fma_fn: extern "C" fn(i32, i32, i32) -> i32 = __lp_fixed32_fma;
        let _inversesqrt_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_inversesqrt;
        let _ldexp_fn: extern "C" fn(i32, i32) -> i32 = __lp_fixed32_ldexp;
        let _log_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_log;
        let _log2_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_log2;
        let _mod_fn: extern "C" fn(i32, i32) -> i32 = __lp_fixed32_mod;
        let _mul_fn: extern "C" fn(i32, i32) -> i32 = __lp_fixed32_mul;
        let _pow_fn: extern "C" fn(i32, i32) -> i32 = __lp_fixed32_pow;
        let _round_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_round;
        let _roundeven_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_roundeven;
        let _sin_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_sin;
        let _sinh_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_sinh;
        let _sqrt_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_sqrt;
        let _tan_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_tan;
        let _tanh_fn: extern "C" fn(i32) -> i32 = __lp_fixed32_tanh;

        // Force these to be included by using them in a way that can't be optimized away
        // We'll use volatile reads to prevent optimization
        let _ = core::ptr::read_volatile(&_acos_fn as *const _);
        let _ = core::ptr::read_volatile(&_acosh_fn as *const _);
        let _ = core::ptr::read_volatile(&_asin_fn as *const _);
        let _ = core::ptr::read_volatile(&_asinh_fn as *const _);
        let _ = core::ptr::read_volatile(&_atan_fn as *const _);
        let _ = core::ptr::read_volatile(&_atan2_fn as *const _);
        let _ = core::ptr::read_volatile(&_atanh_fn as *const _);
        let _ = core::ptr::read_volatile(&_cos_fn as *const _);
        let _ = core::ptr::read_volatile(&_cosh_fn as *const _);
        let _ = core::ptr::read_volatile(&_div_fn as *const _);
        let _ = core::ptr::read_volatile(&_exp_fn as *const _);
        let _ = core::ptr::read_volatile(&_exp2_fn as *const _);
        let _ = core::ptr::read_volatile(&_fma_fn as *const _);
        let _ = core::ptr::read_volatile(&_inversesqrt_fn as *const _);
        let _ = core::ptr::read_volatile(&_ldexp_fn as *const _);
        let _ = core::ptr::read_volatile(&_log_fn as *const _);
        let _ = core::ptr::read_volatile(&_log2_fn as *const _);
        let _ = core::ptr::read_volatile(&_mod_fn as *const _);
        let _ = core::ptr::read_volatile(&_mul_fn as *const _);
        let _ = core::ptr::read_volatile(&_pow_fn as *const _);
        let _ = core::ptr::read_volatile(&_round_fn as *const _);
        let _ = core::ptr::read_volatile(&_roundeven_fn as *const _);
        let _ = core::ptr::read_volatile(&_sin_fn as *const _);
        let _ = core::ptr::read_volatile(&_sinh_fn as *const _);
        let _ = core::ptr::read_volatile(&_sqrt_fn as *const _);
        let _ = core::ptr::read_volatile(&_tan_fn as *const _);
        let _ = core::ptr::read_volatile(&_tanh_fn as *const _);
    }
}
