//! 2D Periodic Simplex Rotational Domain noise function (float implementation - stub).
//!
//! This is a stub implementation that calls the q32 version with conversion.

use crate::builtins::lpfx::generative::psrdnoise::psrdnoise2_q32::__lpfx_psrdnoise2_q32;
use crate::util::q32::Q32;

/// 2D Periodic Simplex Rotational Domain noise function (float version).
///
/// # Arguments
/// * `x` - X coordinate as f32
/// * `y` - Y coordinate as f32
/// * `period_x` - X period as f32 (0 = no tiling)
/// * `period_y` - Y period as f32 (0 = no tiling)
/// * `alpha` - Rotation angle in radians as f32
/// * `gradient_out` - Pointer to output gradient [gx, gy] as f32
/// * `seed` - Seed value for randomization (unused in psrdnoise, kept for consistency)
///
/// # Returns
/// Noise value approximately in range [-1, 1] as f32
#[lpfx_impl_macro::lpfx_impl(
    f32,
    "float lpfx_psrdnoise(vec2 x, vec2 period, float alpha, out vec2 gradient)"
)]
#[unsafe(no_mangle)]
pub extern "C" fn __lpfx_psrdnoise2_f32(
    x: f32,
    y: f32,
    period_x: f32,
    period_y: f32,
    alpha: f32,
    gradient_out: *mut f32,
    seed: u32,
) -> f32 {
    // Convert to q32, call q32 version, convert back
    let x_q32 = Q32::from_f32(x);
    let y_q32 = Q32::from_f32(y);
    let period_x_q32 = Q32::from_f32(period_x);
    let period_y_q32 = Q32::from_f32(period_y);
    let alpha_q32 = Q32::from_f32(alpha);

    let mut gradient_q32 = [0i32; 2];
    let result_fixed = __lpfx_psrdnoise2_q32(
        x_q32.to_fixed(),
        y_q32.to_fixed(),
        period_x_q32.to_fixed(),
        period_y_q32.to_fixed(),
        alpha_q32.to_fixed(),
        gradient_q32.as_mut_ptr(),
        seed,
    );

    // Convert gradient back to f32
    unsafe {
        *gradient_out = Q32::from_fixed(gradient_q32[0]).to_f32();
        *gradient_out.add(1) = Q32::from_fixed(gradient_q32[1]).to_f32();
    }

    Q32::from_fixed(result_fixed).to_f32()
}
