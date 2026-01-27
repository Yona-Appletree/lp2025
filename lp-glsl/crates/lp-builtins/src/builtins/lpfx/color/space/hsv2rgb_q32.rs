//! Convert HSV color space to RGB.
//!
//! Converts colors from HSV (Hue, Saturation, Value) color space to RGB color space.
//! This implementation follows the algorithm from lygia.

use crate::builtins::lpfx::color::space::hue2rgb_q32::lpfx_hue2rgb_q32;
use crate::util::q32::Q32;
use crate::util::vec3_q32::Vec3Q32;
use crate::util::vec4_q32::Vec4Q32;

/// Convert HSV color to RGB color.
///
/// Converts a color from HSV color space to RGB color space.
///
/// # Arguments
/// * `hsv` - HSV color as Vec3Q32 (H, S, V components in range [0, 1])
///
/// # Returns
/// RGB color as Vec3Q32 with components in range [0, 1]
#[inline(always)]
pub fn lpfx_hsv2rgb_q32(hsv: Vec3Q32) -> Vec3Q32 {
    // Algorithm from lygia: ((hue2rgb(hsv.x) - 1.0) * hsv.y + 1.0) * hsv.z
    let hue_rgb = lpfx_hue2rgb_q32(hsv.x);
    let rgb_minus_one = hue_rgb - Vec3Q32::one();
    let rgb_scaled = rgb_minus_one * hsv.y + Vec3Q32::one();
    rgb_scaled * hsv.z
}

/// Convert HSV color to RGB color (with alpha channel preserved).
///
/// Converts a color from HSV color space to RGB color space, preserving
/// the alpha channel.
///
/// # Arguments
/// * `hsv` - HSV color as Vec4Q32 (H, S, V, A components, H/S/V in range [0, 1])
///
/// # Returns
/// RGBA color as Vec4Q32 with RGB components in range [0, 1], alpha preserved
#[inline(always)]
pub fn lpfx_hsv2rgb_vec4_q32(hsv: Vec4Q32) -> Vec4Q32 {
    let hsv_vec3 = Vec3Q32::new(hsv.x, hsv.y, hsv.z);
    let rgb_vec3 = lpfx_hsv2rgb_q32(hsv_vec3);
    Vec4Q32::new(rgb_vec3.x, rgb_vec3.y, rgb_vec3.z, hsv.w)
}

/// Convert HSV color to RGB color (extern C wrapper for compiler).
///
/// # Arguments
/// * `x` - H component as i32 (Q32 fixed-point)
/// * `y` - S component as i32 (Q32 fixed-point)
/// * `z` - V component as i32 (Q32 fixed-point)
///
/// # Returns
/// R component as i32 (Q32 fixed-point)
#[lpfx_impl_macro::lpfx_impl(q32, "vec3 lpfx_hsv2rgb(vec3 hsv)")]
#[unsafe(no_mangle)]
pub extern "C" fn __lpfx_hsv2rgb_q32(x: i32, y: i32, z: i32) -> i32 {
    let hsv = Vec3Q32::new(Q32::from_fixed(x), Q32::from_fixed(y), Q32::from_fixed(z));
    let result = lpfx_hsv2rgb_q32(hsv);
    result.x.to_fixed()
}

/// Convert HSV color to RGB color with alpha (extern C wrapper for compiler).
///
/// # Arguments
/// * `x` - H component as i32 (Q32 fixed-point)
/// * `y` - S component as i32 (Q32 fixed-point)
/// * `z` - V component as i32 (Q32 fixed-point)
/// * `w` - A component as i32 (Q32 fixed-point)
///
/// # Returns
/// R component as i32 (Q32 fixed-point)
#[lpfx_impl_macro::lpfx_impl(q32, "vec4 lpfx_hsv2rgb(vec4 hsv)")]
#[unsafe(no_mangle)]
pub extern "C" fn __lpfx_hsv2rgb_vec4_q32(x: i32, y: i32, z: i32, w: i32) -> i32 {
    let hsv = Vec4Q32::new(
        Q32::from_fixed(x),
        Q32::from_fixed(y),
        Q32::from_fixed(z),
        Q32::from_fixed(w),
    );
    let result = lpfx_hsv2rgb_vec4_q32(hsv);
    result.x.to_fixed()
}
