//! Convert hue value to RGB color (float implementation - stub).
//!
//! This is a stub implementation that will be replaced with a proper float implementation later.
//! For now, it calls the q32 version with conversion.

use crate::builtins::lpfx::color::space::hue2rgb_q32::__lpfx_hue2rgb_q32;
use crate::util::q32::Q32;

/// Convert hue value to RGB color (extern C wrapper for compiler).
///
/// # Arguments
/// * `hue` - Hue value as f32
///
/// # Returns
/// R component as f32
#[lpfx_impl_macro::lpfx_impl(f32, "vec3 lpfx_hue2rgb(float hue)")]
#[unsafe(no_mangle)]
pub extern "C" fn __lpfx_hue2rgb_f32(hue: f32) -> f32 {
    // Stub: convert to q32, call q32 version, convert back
    let hue_q32 = Q32::from_f32(hue);
    let result_fixed = __lpfx_hue2rgb_q32(hue_q32.to_fixed());
    Q32::from_fixed(result_fixed).to_f32()
}
