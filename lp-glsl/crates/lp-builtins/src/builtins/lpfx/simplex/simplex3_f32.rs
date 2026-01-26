//! 3D Simplex noise function (float implementation - stub).
//!
//! This is a stub implementation that will be replaced with a proper float implementation later.
//! For now, it calls the q32 version with conversion.

use crate::builtins::lpfx::simplex::simplex3_q32::__lpfx_simplex3_q32;
use crate::util::q32::Q32;

/// 3D Simplex noise function (float version).
///
/// # Arguments
/// * `x` - X coordinate as f32
/// * `y` - Y coordinate as f32
/// * `z` - Z coordinate as f32
/// * `seed` - Seed value for randomization
///
/// # Returns
/// Noise value approximately in range [-1, 1] as f32
#[allow(unknown_attributes)]
#[lpfx_impl(f32, "float lpfx_simplex3(vec3 p, uint seed)")]
#[unsafe(no_mangle)]
pub extern "C" fn __lpfx_simplex3_f32(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    // Stub: convert to fixed32, call q32 version, convert back
    let x_q32 = Q32::from_f32(x);
    let y_q32 = Q32::from_f32(y);
    let z_q32 = Q32::from_f32(z);
    let result_fixed =
        __lpfx_simplex3_q32(x_q32.to_fixed(), y_q32.to_fixed(), z_q32.to_fixed(), seed);
    Q32::from_fixed(result_fixed).to_f32()
}
