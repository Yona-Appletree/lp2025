//! Convert RGB color space to HSV.
//!
//! Converts colors from RGB color space to HSV (Hue, Saturation, Value) color space.
//! This implementation follows Sam Hocevar's algorithm from lygia.

use crate::util::q32::Q32;
use crate::util::vec3_q32::Vec3Q32;
use crate::util::vec4_q32::Vec4Q32;

/// Epsilon constant to avoid division by zero.
/// Using minimum representable Q32 value (1 in Q16.16 format = 1/65536 ≈ 0.000015).
const HCV_EPSILON_Q32: Q32 = Q32(1);

/// Fixed-point constants for rgb2hsv calculation
const SIX: Q32 = Q32(0x00060000); // 6.0 in Q16.16

/// K constant vector for rgb2hsv algorithm
/// K = vec4(0., -0.33333333333333333333, 0.6666666666666666666, -1.0)
const K_X: Q32 = Q32::ZERO;
const K_Y: Q32 = Q32::from_fixed(-21845); // -0.33333333333333333333 * 65536 ≈ -21845
const K_Z: Q32 = Q32::from_fixed(43690); // 0.6666666666666666666 * 65536 ≈ 43690
const K_W: Q32 = Q32::from_fixed(-65536); // -1.0 * 65536 = -65536

/// Convert RGB color to HSV color.
///
/// Converts a color from RGB color space to HSV color space.
/// Algorithm from Sam Hocevar: http://lolengine.net/blog/2013/07/27/rgb-to-hsv-in-glsl
///
/// # Arguments
/// * `rgb` - RGB color as Vec3Q32 with components in range [0, 1]
///
/// # Returns
/// HSV color as Vec3Q32 (H, S, V components in range [0, 1])
#[inline(always)]
pub fn lpfx_rgb2hsv_q32(rgb: Vec3Q32) -> Vec3Q32 {
    // Algorithm from lygia (Sam Hocevar's implementation)
    // vec4 K = vec4(0., -0.33333333333333333333, 0.6666666666666666666, -1.0);
    // vec4 p = c.g < c.b ? vec4(c.bg, K.wz) : vec4(c.gb, K.xy);
    // vec4 q = c.r < p.x ? vec4(p.xyw, c.r) : vec4(c.r, p.yzx);
    // float d = q.x - min(q.w, q.y);
    // return vec3(abs(q.z + (q.w - q.y) / (6. * d + HCV_EPSILON)),
    //             d / (q.x + HCV_EPSILON),
    //             q.x);

    let c = rgb;
    let p = if c.y < c.z {
        // p = vec4(c.bg, K.wz) = vec4(c.z, c.y, K_W, K_Z)
        Vec4Q32::new(c.z, c.y, K_W, K_Z)
    } else {
        // p = vec4(c.gb, K.xy) = vec4(c.y, c.z, K_X, K_Y)
        Vec4Q32::new(c.y, c.z, K_X, K_Y)
    };

    let q = if c.x < p.x {
        // q = vec4(p.xyw, c.r) = vec4(p.x, p.y, p.w, c.x)
        Vec4Q32::new(p.x, p.y, p.w, c.x)
    } else {
        // q = vec4(c.r, p.yzx) = vec4(c.x, p.y, p.z, p.x)
        Vec4Q32::new(c.x, p.y, p.z, p.x)
    };

    let d = q.x - q.w.min(q.y);
    let h = (q.z + (q.w - q.y) / (SIX * d + HCV_EPSILON_Q32)).abs();
    let s = d / (q.x + HCV_EPSILON_Q32);
    let v = q.x;

    Vec3Q32::new(h, s, v)
}

/// Convert RGB color to HSV color (with alpha channel preserved).
///
/// Converts a color from RGB color space to HSV color space, preserving
/// the alpha channel.
///
/// # Arguments
/// * `rgb` - RGBA color as Vec4Q32 with RGB components in range [0, 1]
///
/// # Returns
/// HSVA color as Vec4Q32 (H, S, V components in range [0, 1], alpha preserved)
#[inline(always)]
pub fn lpfx_rgb2hsv_vec4_q32(rgb: Vec4Q32) -> Vec4Q32 {
    let rgb_vec3 = Vec3Q32::new(rgb.x, rgb.y, rgb.z);
    let hsv_vec3 = lpfx_rgb2hsv_q32(rgb_vec3);
    Vec4Q32::new(hsv_vec3.x, hsv_vec3.y, hsv_vec3.z, rgb.w)
}

/// Convert RGB color to HSV color (extern C wrapper for compiler).
///
/// # Arguments
/// * `x` - R component as i32 (Q32 fixed-point)
/// * `y` - G component as i32 (Q32 fixed-point)
/// * `z` - B component as i32 (Q32 fixed-point)
///
/// # Returns
/// H component as i32 (Q32 fixed-point)
#[lpfx_impl_macro::lpfx_impl(q32, "vec3 lpfx_rgb2hsv(vec3 rgb)")]
#[unsafe(no_mangle)]
pub extern "C" fn __lpfx_rgb2hsv_q32(x: i32, y: i32, z: i32) -> i32 {
    let rgb = Vec3Q32::new(Q32::from_fixed(x), Q32::from_fixed(y), Q32::from_fixed(z));
    let result = lpfx_rgb2hsv_q32(rgb);
    result.x.to_fixed()
}

/// Convert RGB color to HSV color with alpha (extern C wrapper for compiler).
///
/// # Arguments
/// * `x` - R component as i32 (Q32 fixed-point)
/// * `y` - G component as i32 (Q32 fixed-point)
/// * `z` - B component as i32 (Q32 fixed-point)
/// * `w` - A component as i32 (Q32 fixed-point)
///
/// # Returns
/// H component as i32 (Q32 fixed-point)
#[lpfx_impl_macro::lpfx_impl(q32, "vec4 lpfx_rgb2hsv(vec4 rgb)")]
#[unsafe(no_mangle)]
pub extern "C" fn __lpfx_rgb2hsv_vec4_q32(x: i32, y: i32, z: i32, w: i32) -> i32 {
    let rgb = Vec4Q32::new(
        Q32::from_fixed(x),
        Q32::from_fixed(y),
        Q32::from_fixed(z),
        Q32::from_fixed(w),
    );
    let result = lpfx_rgb2hsv_vec4_q32(rgb);
    result.x.to_fixed()
}
