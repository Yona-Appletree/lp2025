//! Convert RGB color space to HSV (float implementation - stub).
//!
//! This is a stub implementation that will be replaced with a proper float implementation later.
//! For now, it calls the q32 version with conversion.

use crate::builtins::lpfx::color::space::rgb2hsv_q32::__lpfx_rgb2hsv_q32;
use crate::builtins::lpfx::color::space::rgb2hsv_q32::__lpfx_rgb2hsv_vec4_q32;
use crate::util::q32::Q32;

/// Convert RGB color to HSV color (extern C wrapper for compiler).
///
/// # Arguments
/// * `x` - R component as f32
/// * `y` - G component as f32
/// * `z` - B component as f32
///
/// # Returns
/// H component as f32
#[lpfx_impl_macro::lpfx_impl(f32, "vec3 lpfx_rgb2hsv(vec3 rgb)")]
#[unsafe(no_mangle)]
pub extern "C" fn __lpfx_rgb2hsv_f32(x: f32, y: f32, z: f32) -> f32 {
    // Stub: convert to q32, call q32 version, convert back
    let x_q32 = Q32::from_f32(x);
    let y_q32 = Q32::from_f32(y);
    let z_q32 = Q32::from_f32(z);
    let result_fixed = __lpfx_rgb2hsv_q32(x_q32.to_fixed(), y_q32.to_fixed(), z_q32.to_fixed());
    Q32::from_fixed(result_fixed).to_f32()
}

/// Convert RGB color to HSV color with alpha (extern C wrapper for compiler).
///
/// # Arguments
/// * `x` - R component as f32
/// * `y` - G component as f32
/// * `z` - B component as f32
/// * `w` - A component as f32
///
/// # Returns
/// H component as f32
#[lpfx_impl_macro::lpfx_impl(f32, "vec4 lpfx_rgb2hsv(vec4 rgb)")]
#[unsafe(no_mangle)]
pub extern "C" fn __lpfx_rgb2hsv_vec4_f32(x: f32, y: f32, z: f32, w: f32) -> f32 {
    // Stub: convert to q32, call q32 version, convert back
    let x_q32 = Q32::from_f32(x);
    let y_q32 = Q32::from_f32(y);
    let z_q32 = Q32::from_f32(z);
    let w_q32 = Q32::from_f32(w);
    let result_fixed = __lpfx_rgb2hsv_vec4_q32(
        x_q32.to_fixed(),
        y_q32.to_fixed(),
        z_q32.to_fixed(),
        w_q32.to_fixed(),
    );
    Q32::from_fixed(result_fixed).to_f32()
}
