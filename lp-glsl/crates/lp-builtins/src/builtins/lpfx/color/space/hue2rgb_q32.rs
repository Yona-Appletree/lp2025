//! Convert hue value to RGB color.
//!
//! Converts a hue value (0-1) to an RGB vec3 color. This is a helper function
//! used by HSV to RGB conversion.

use crate::builtins::lpfx::math::saturate_q32::lpfx_saturate_vec3_q32;
use crate::util::q32::Q32;
use crate::util::vec3_q32::Vec3Q32;

/// Fixed-point constants for hue2rgb calculation
const TWO: Q32 = Q32(0x00020000); // 2.0 in Q16.16
const THREE: Q32 = Q32(0x00030000); // 3.0 in Q16.16
const FOUR: Q32 = Q32(0x00040000); // 4.0 in Q16.16
const SIX: Q32 = Q32(0x00060000); // 6.0 in Q16.16

/// Convert hue value to RGB color.
///
/// Converts a hue value in the range [0, 1] to an RGB vec3 color.
/// The hue value wraps around (hue values > 1.0 are handled via fract).
///
/// # Arguments
/// * `hue` - Hue value in range [0, 1] (Q32 fixed-point)
///
/// # Returns
/// RGB color as Vec3Q32 with components in range [0, 1]
#[inline(always)]
pub fn lpfx_hue2rgb_q32(hue: Q32) -> Vec3Q32 {
    // Algorithm from lygia: uses abs() and arithmetic to compute RGB from hue
    // R = abs(hue * 6.0 - 3.0) - 1.0
    // G = 2.0 - abs(hue * 6.0 - 2.0)
    // B = 2.0 - abs(hue * 6.0 - 4.0)
    let hue_times_six = hue * SIX;
    let r = (hue_times_six - THREE).abs() - Q32::ONE;
    let g = TWO - (hue_times_six - TWO).abs();
    let b = TWO - (hue_times_six - FOUR).abs();

    let rgb = Vec3Q32::new(r, g, b);
    lpfx_saturate_vec3_q32(rgb)
}

/// Convert hue value to RGB color (extern C wrapper for compiler).
///
/// # Arguments
/// * `hue` - Hue value as i32 (Q32 fixed-point)
///
/// # Returns
/// R component as i32 (Q32 fixed-point)
#[lpfx_impl_macro::lpfx_impl(q32, "vec3 lpfx_hue2rgb(float hue)")]
#[unsafe(no_mangle)]
pub extern "C" fn __lpfx_hue2rgb_q32(hue: i32) -> i32 {
    let result = lpfx_hue2rgb_q32(Q32::from_fixed(hue));
    result.x.to_fixed()
}
