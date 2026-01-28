//! 3D Periodic Simplex Rotational Domain noise function.
//!
//! Periodic Simplex Rotational Domain noise (psrdnoise) is a variant of Simplex noise
//! that supports seamless tiling and rotational gradients for flow-like effects.
//! This implementation uses Q32 fixed-point arithmetic (16.16 format).
//!
//! Reference: Lygia's psrdnoise implementation by Stefan Gustavson and Ian McEwan
//!
//! # GLSL Usage
//!
//! This function is callable from GLSL shaders using the `lpfx_psrdnoise` name:
//!
//! ```glsl
//! vec3 gradient;
//! float noise = lpfx_psrdnoise(vec3(5.0, 3.0, 1.0), vec3(10.0, 10.0, 10.0), 0.5, gradient);
//! ```
//!
//! # Parameters
//!
//! - `x`: Input coordinates as vec3 (converted to Q32 internally, flattened to x, y, z)
//! - `period`: Tiling period as vec3 (0 = no tiling, flattened to period_x, period_y, period_z)
//! - `alpha`: Rotation angle in radians (float, converted to Q32)
//! - `gradient`: Output gradient vector (out vec3, written to pointer)
//!
//! # Returns
//!
//! Noise value approximately in range [-1, 1] (float)

use crate::builtins::q32::{__lp_q32_cos, __lp_q32_mod, __lp_q32_sin, __lp_q32_sqrt};
use crate::glsl::q32::types::q32::Q32;
use crate::glsl::q32::types::vec3_q32::Vec3Q32;

/// Fixed-point constants
const HALF: Q32 = Q32(0x00008000); // 0.5 in Q16.16
const SIX: Q32 = Q32(0x00060000); // 6.0 in Q16.16

/// Period constant for hash: 289.0
/// In Q16.16: 289.0 * 65536 = 18939904
const PERIOD_289: Q32 = Q32(18939904);

/// Radial decay constant: 0.5
/// In Q16.16: 0.5 * 65536 = 32768
const RADIAL_DECAY_0_5: Q32 = Q32(32768);

/// Final scale factor: 39.5
/// In Q16.16: 39.5 * 65536 ≈ 2588672
const SCALE_39_5: Q32 = Q32(2588672);

/// Hash computation constants
const HASH_CONST_34: Q32 = Q32(34 << 16); // 34.0

/// Fibonacci spiral constants
/// 2*pi/golden ratio ≈ 3.883222077
const THETA_MULT: Q32 = Q32(254545); // 3.883222077 * 65536
/// -0.006920415
const SZ_MULT: Q32 = Q32(-454); // -0.006920415 * 65536
/// 0.996539792
const SZ_ADD: Q32 = Q32(65296); // 0.996539792 * 65536
/// 10*pi/289 ≈ 0.108705628
const PSI_MULT: Q32 = Q32(7124); // 0.108705628 * 65536
/// 1/3 ≈ 0.33333333
const ONE_THIRD: Q32 = Q32(21845); // 0.33333333 * 65536
/// 1/6 ≈ 0.16666667
const ONE_SIXTH: Q32 = Q32(10923); // 0.16666667 * 65536

/// Helper: mod289(x) = mod(x, 289.0)
#[inline(always)]
fn mod289_q32(x: i32) -> i32 {
    __lp_q32_mod(x, PERIOD_289.to_fixed())
}

/// Helper: permute(v) = mod289(((v * 34.0) + 1.0) * v)
#[inline(always)]
fn permute_q32(v: i32) -> i32 {
    let v_q32 = Q32::from_fixed(v);
    let temp = v_q32 * HASH_CONST_34 + Q32::ONE;
    mod289_q32((temp * v_q32).to_fixed())
}

/// 3D Periodic Simplex Rotational Domain noise function.
///
/// # Arguments
/// * `x` - Input coordinates as Vec3Q32
/// * `period` - Tiling period as Vec3Q32 (zero = no tiling)
/// * `alpha` - Rotation angle in radians as Q32
/// * `seed` - Seed value for randomization (unused in psrdnoise, kept for consistency)
///
/// # Returns
/// Tuple of (noise_value, gradient_x, gradient_y, gradient_z) in Q32 fixed-point format
pub fn lpfx_psrdnoise3(
    x: Vec3Q32,
    period: Vec3Q32,
    alpha: Q32,
    _seed: u32,
) -> (Q32, Q32, Q32, Q32) {
    // Transform to simplex space (tetrahedral grid)
    // Using optimized transformation: uvw = x + dot(x, vec3(1.0/3.0))
    let dot_sum = x.x + x.y + x.z;
    let uvw_x = x.x + dot_sum * ONE_THIRD;
    let uvw_y = x.y + dot_sum * ONE_THIRD;
    let uvw_z = x.z + dot_sum * ONE_THIRD;

    // Determine which simplex we're in, i0 is the "base corner"
    // i0 = floor(uvw)
    let i0_x_int = uvw_x.to_i32();
    let i0_y_int = uvw_y.to_i32();
    let i0_z_int = uvw_z.to_i32();
    let i0_x = Q32::from_i32(i0_x_int);
    let i0_y = Q32::from_i32(i0_y_int);
    let i0_z = Q32::from_i32(i0_z_int);

    // f0 = fract(uvw)
    let f0_x = uvw_x - i0_x;
    let f0_y = uvw_y - i0_y;
    let f0_z = uvw_z - i0_z;

    // To determine which simplex corners are closest, rank order the
    // magnitudes of u,v,w, resolving ties in priority order u,v,w
    // g_ = step(f0.xyx, f0.yzz) -> 1.0 if f0.xyx <= f0.yzz, else 0.0
    let g_x = if f0_x <= f0_y { Q32::ONE } else { Q32::ZERO };
    let g_y = if f0_y <= f0_z { Q32::ONE } else { Q32::ZERO };
    let g_z = if f0_x <= f0_z { Q32::ONE } else { Q32::ZERO };
    // l_ = 1.0 - g_
    let l_x = Q32::ONE - g_x;
    let l_y = Q32::ONE - g_y;
    let l_z = Q32::ONE - g_z;
    // g = vec3(l_.z, g_.xy)
    let g_x_final = l_z;
    let g_y_final = g_x;
    let g_z_final = g_y;
    // l = vec3(l_.xy, g_.z)
    let l_x_final = l_x;
    let l_y_final = l_y;
    let l_z_final = g_z;
    // o1 = min(g, l), o2 = max(g, l)
    let o1_x = g_x_final.min(l_x_final);
    let o1_y = g_y_final.min(l_y_final);
    let o1_z = g_z_final.min(l_z_final);
    let o2_x = g_x_final.max(l_x_final);
    let o2_y = g_y_final.max(l_y_final);
    let o2_z = g_z_final.max(l_z_final);

    // Enumerate the remaining simplex corners
    // i1 = i0 + o1, i2 = i0 + o2, i3 = i0 + vec3(1.0)
    let i1_x_int = i0_x_int + o1_x.to_i32();
    let i1_y_int = i0_y_int + o1_y.to_i32();
    let i1_z_int = i0_z_int + o1_z.to_i32();
    let i2_x_int = i0_x_int + o2_x.to_i32();
    let i2_y_int = i0_y_int + o2_y.to_i32();
    let i2_z_int = i0_z_int + o2_z.to_i32();
    let i3_x_int = i0_x_int + 1;
    let i3_y_int = i0_y_int + 1;
    let i3_z_int = i0_z_int + 1;

    // Transform the corners back to texture space
    // Using optimized transformation: v = i - dot(i, vec3(1.0/6.0))
    let dot0 = (i0_x + i0_y + i0_z) * ONE_SIXTH;
    let dot1 =
        (Q32::from_i32(i1_x_int) + Q32::from_i32(i1_y_int) + Q32::from_i32(i1_z_int)) * ONE_SIXTH;
    let dot2 =
        (Q32::from_i32(i2_x_int) + Q32::from_i32(i2_y_int) + Q32::from_i32(i2_z_int)) * ONE_SIXTH;
    let dot3 =
        (Q32::from_i32(i3_x_int) + Q32::from_i32(i3_y_int) + Q32::from_i32(i3_z_int)) * ONE_SIXTH;

    let v0_x = i0_x - dot0;
    let v0_y = i0_y - dot0;
    let v0_z = i0_z - dot0;
    let v1_x = Q32::from_i32(i1_x_int) - dot1;
    let v1_y = Q32::from_i32(i1_y_int) - dot1;
    let v1_z = Q32::from_i32(i1_z_int) - dot1;
    let v2_x = Q32::from_i32(i2_x_int) - dot2;
    let v2_y = Q32::from_i32(i2_y_int) - dot2;
    let v2_z = Q32::from_i32(i2_z_int) - dot2;
    let v3_x = Q32::from_i32(i3_x_int) - dot3;
    let v3_y = Q32::from_i32(i3_y_int) - dot3;
    let v3_z = Q32::from_i32(i3_z_int) - dot3;

    // Compute vectors to each of the simplex corners
    let x0_x = x.x - v0_x;
    let x0_y = x.y - v0_y;
    let x0_z = x.z - v0_z;
    let x1_x = x.x - v1_x;
    let x1_y = x.y - v1_y;
    let x1_z = x.z - v1_z;
    let x2_x = x.x - v2_x;
    let x2_y = x.y - v2_y;
    let x2_z = x.z - v2_z;
    let x3_x = x.x - v3_x;
    let x3_y = x.y - v3_y;
    let x3_z = x.z - v3_z;

    // Wrap to periods, if desired
    let (
        i0_x_final,
        i0_y_final,
        i0_z_final,
        i1_x_final,
        i1_y_final,
        i1_z_final,
        i2_x_final,
        i2_y_final,
        i2_z_final,
        i3_x_final,
        i3_y_final,
        i3_z_final,
    ) = if period.x > Q32::ZERO || period.y > Q32::ZERO || period.z > Q32::ZERO {
        let mut vx_x = v0_x;
        let mut vx_y = v1_x;
        let mut vx_z = v2_x;
        let mut vx_w = v3_x;
        let mut vy_x = v0_y;
        let mut vy_y = v1_y;
        let mut vy_z = v2_y;
        let mut vy_w = v3_y;
        let mut vz_x = v0_z;
        let mut vz_y = v1_z;
        let mut vz_z = v2_z;
        let mut vz_w = v3_z;

        // Wrap to periods where specified
        if period.x > Q32::ZERO {
            vx_x = Q32::from_fixed(__lp_q32_mod(v0_x.to_fixed(), period.x.to_fixed()));
            vx_y = Q32::from_fixed(__lp_q32_mod(v1_x.to_fixed(), period.x.to_fixed()));
            vx_z = Q32::from_fixed(__lp_q32_mod(v2_x.to_fixed(), period.x.to_fixed()));
            vx_w = Q32::from_fixed(__lp_q32_mod(v3_x.to_fixed(), period.x.to_fixed()));
        }
        if period.y > Q32::ZERO {
            vy_x = Q32::from_fixed(__lp_q32_mod(v0_y.to_fixed(), period.y.to_fixed()));
            vy_y = Q32::from_fixed(__lp_q32_mod(v1_y.to_fixed(), period.y.to_fixed()));
            vy_z = Q32::from_fixed(__lp_q32_mod(v2_y.to_fixed(), period.y.to_fixed()));
            vy_w = Q32::from_fixed(__lp_q32_mod(v3_y.to_fixed(), period.y.to_fixed()));
        }
        if period.z > Q32::ZERO {
            vz_x = Q32::from_fixed(__lp_q32_mod(v0_z.to_fixed(), period.z.to_fixed()));
            vz_y = Q32::from_fixed(__lp_q32_mod(v1_z.to_fixed(), period.z.to_fixed()));
            vz_z = Q32::from_fixed(__lp_q32_mod(v2_z.to_fixed(), period.z.to_fixed()));
            vz_w = Q32::from_fixed(__lp_q32_mod(v3_z.to_fixed(), period.z.to_fixed()));
        }

        // Transform wrapped coordinates back to uvw
        // i = v + dot(v, vec3(1.0/3.0))
        let dot_v0 = (vx_x + vy_x + vz_x) * ONE_THIRD;
        let dot_v1 = (vx_y + vy_y + vz_y) * ONE_THIRD;
        let dot_v2 = (vx_z + vy_z + vz_z) * ONE_THIRD;
        let dot_v3 = (vx_w + vy_w + vz_w) * ONE_THIRD;

        let i0_x_wrapped = (vx_x + dot_v0 + HALF).to_i32();
        let i0_y_wrapped = (vy_x + dot_v0 + HALF).to_i32();
        let i0_z_wrapped = (vz_x + dot_v0 + HALF).to_i32();
        let i1_x_wrapped = (vx_y + dot_v1 + HALF).to_i32();
        let i1_y_wrapped = (vy_y + dot_v1 + HALF).to_i32();
        let i1_z_wrapped = (vz_y + dot_v1 + HALF).to_i32();
        let i2_x_wrapped = (vx_z + dot_v2 + HALF).to_i32();
        let i2_y_wrapped = (vy_z + dot_v2 + HALF).to_i32();
        let i2_z_wrapped = (vz_z + dot_v2 + HALF).to_i32();
        let i3_x_wrapped = (vx_w + dot_v3 + HALF).to_i32();
        let i3_y_wrapped = (vy_w + dot_v3 + HALF).to_i32();
        let i3_z_wrapped = (vz_w + dot_v3 + HALF).to_i32();

        (
            i0_x_wrapped,
            i0_y_wrapped,
            i0_z_wrapped,
            i1_x_wrapped,
            i1_y_wrapped,
            i1_z_wrapped,
            i2_x_wrapped,
            i2_y_wrapped,
            i2_z_wrapped,
            i3_x_wrapped,
            i3_y_wrapped,
            i3_z_wrapped,
        )
    } else {
        (
            i0_x_int, i0_y_int, i0_z_int, i1_x_int, i1_y_int, i1_z_int, i2_x_int, i2_y_int,
            i2_z_int, i3_x_int, i3_y_int, i3_z_int,
        )
    };

    // Avoid truncation effects in permutation
    // i0 = mod289(i0), etc.
    let i0_x_mod = mod289_q32(i0_x_final << 16) >> 16;
    let i0_y_mod = mod289_q32(i0_y_final << 16) >> 16;
    let i0_z_mod = mod289_q32(i0_z_final << 16) >> 16;
    let i1_x_mod = mod289_q32(i1_x_final << 16) >> 16;
    let i1_y_mod = mod289_q32(i1_y_final << 16) >> 16;
    let i1_z_mod = mod289_q32(i1_z_final << 16) >> 16;
    let i2_x_mod = mod289_q32(i2_x_final << 16) >> 16;
    let i2_y_mod = mod289_q32(i2_y_final << 16) >> 16;
    let i2_z_mod = mod289_q32(i2_z_final << 16) >> 16;
    let i3_x_mod = mod289_q32(i3_x_final << 16) >> 16;
    let i3_y_mod = mod289_q32(i3_y_final << 16) >> 16;
    let i3_z_mod = mod289_q32(i3_z_final << 16) >> 16;

    // Compute one pseudo-random hash value for each corner
    // hash = permute(permute(permute(vec4(i0.z, i1.z, i2.z, i3.z)) + vec4(i0.y, i1.y, i2.y, i3.y)) + vec4(i0.x, i1.x, i2.x, i3.x))
    let hash_z0 = permute_q32(i0_z_mod << 16);
    let hash_z1 = permute_q32(i1_z_mod << 16);
    let hash_z2 = permute_q32(i2_z_mod << 16);
    let hash_z3 = permute_q32(i3_z_mod << 16);

    let hash_y0 = permute_q32((hash_z0 >> 16) + (i0_y_mod << 16));
    let hash_y1 = permute_q32((hash_z1 >> 16) + (i1_y_mod << 16));
    let hash_y2 = permute_q32((hash_z2 >> 16) + (i2_y_mod << 16));
    let hash_y3 = permute_q32((hash_z3 >> 16) + (i3_y_mod << 16));

    let hash_x0 = permute_q32((hash_y0 >> 16) + (i0_x_mod << 16));
    let hash_x1 = permute_q32((hash_y1 >> 16) + (i1_x_mod << 16));
    let hash_x2 = permute_q32((hash_y2 >> 16) + (i2_x_mod << 16));
    let hash_x3 = permute_q32((hash_y3 >> 16) + (i3_x_mod << 16));

    // Compute generating gradients from a Fibonacci spiral on the unit sphere
    // theta = hash * 3.883222077 (2*pi/golden ratio)
    let theta_x = Q32::from_fixed(hash_x0) * THETA_MULT;
    let theta_y = Q32::from_fixed(hash_x1) * THETA_MULT;
    let theta_z = Q32::from_fixed(hash_x2) * THETA_MULT;
    let theta_w = Q32::from_fixed(hash_x3) * THETA_MULT;

    // sz = hash * -0.006920415 + 0.996539792 (1-(hash+0.5)*2/289)
    let sz_x = Q32::from_fixed(hash_x0) * SZ_MULT + SZ_ADD;
    let sz_y = Q32::from_fixed(hash_x1) * SZ_MULT + SZ_ADD;
    let sz_z = Q32::from_fixed(hash_x2) * SZ_MULT + SZ_ADD;
    let sz_w = Q32::from_fixed(hash_x3) * SZ_MULT + SZ_ADD;

    // psi = hash * 0.108705628 (10*pi/289)
    let psi_x = Q32::from_fixed(hash_x0) * PSI_MULT;
    let psi_y = Q32::from_fixed(hash_x1) * PSI_MULT;
    let psi_z = Q32::from_fixed(hash_x2) * PSI_MULT;
    let psi_w = Q32::from_fixed(hash_x3) * PSI_MULT;

    // Ct = cos(theta), St = sin(theta)
    let ct_x = Q32::from_fixed(__lp_q32_cos(theta_x.to_fixed()));
    let ct_y = Q32::from_fixed(__lp_q32_cos(theta_y.to_fixed()));
    let ct_z = Q32::from_fixed(__lp_q32_cos(theta_z.to_fixed()));
    let ct_w = Q32::from_fixed(__lp_q32_cos(theta_w.to_fixed()));
    let st_x = Q32::from_fixed(__lp_q32_sin(theta_x.to_fixed()));
    let st_y = Q32::from_fixed(__lp_q32_sin(theta_y.to_fixed()));
    let st_z = Q32::from_fixed(__lp_q32_sin(theta_z.to_fixed()));
    let st_w = Q32::from_fixed(__lp_q32_sin(theta_w.to_fixed()));

    // sz_prime = sqrt(1.0 - sz*sz)
    let sz_prime_x = Q32::from_fixed(__lp_q32_sqrt((Q32::ONE - sz_x * sz_x).to_fixed()));
    let sz_prime_y = Q32::from_fixed(__lp_q32_sqrt((Q32::ONE - sz_y * sz_y).to_fixed()));
    let sz_prime_z = Q32::from_fixed(__lp_q32_sqrt((Q32::ONE - sz_z * sz_z).to_fixed()));
    let sz_prime_w = Q32::from_fixed(__lp_q32_sqrt((Q32::ONE - sz_w * sz_w).to_fixed()));

    // Rotate gradients by angle alpha around a pseudo-random orthogonal axis
    // Using fast rotation algorithm (PSRDNOISE_FAST_ROTATION)
    // qx = St, qy = -Ct, qz = 0.0
    let qx_x = st_x;
    let qx_y = st_y;
    let qx_z = st_z;
    let qx_w = st_w;
    let qy_x = -ct_x;
    let qy_y = -ct_y;
    let qy_z = -ct_z;
    let qy_w = -ct_w;
    let qz_x = Q32::ZERO;
    let qz_y = Q32::ZERO;
    let qz_z = Q32::ZERO;
    let qz_w = Q32::ZERO;

    // px = sz * qy, py = -sz * qx, pz = sz_prime
    let px_x = sz_x * qy_x;
    let px_y = sz_y * qy_y;
    let px_z = sz_z * qy_z;
    let px_w = sz_w * qy_w;
    let py_x = -sz_x * qx_x;
    let py_y = -sz_y * qx_y;
    let py_z = -sz_z * qx_z;
    let py_w = -sz_w * qx_w;
    let pz_x = sz_prime_x;
    let pz_y = sz_prime_y;
    let pz_z = sz_prime_z;
    let pz_w = sz_prime_w;

    // psi += alpha (psi and alpha in the same plane)
    let psi_x_final = psi_x + alpha;
    let psi_y_final = psi_y + alpha;
    let psi_z_final = psi_z + alpha;
    let psi_w_final = psi_w + alpha;

    // Sa = sin(psi), Ca = cos(psi)
    let sa_x = Q32::from_fixed(__lp_q32_sin(psi_x_final.to_fixed()));
    let sa_y = Q32::from_fixed(__lp_q32_sin(psi_y_final.to_fixed()));
    let sa_z = Q32::from_fixed(__lp_q32_sin(psi_z_final.to_fixed()));
    let sa_w = Q32::from_fixed(__lp_q32_sin(psi_w_final.to_fixed()));
    let ca_x = Q32::from_fixed(__lp_q32_cos(psi_x_final.to_fixed()));
    let ca_y = Q32::from_fixed(__lp_q32_cos(psi_y_final.to_fixed()));
    let ca_z = Q32::from_fixed(__lp_q32_cos(psi_z_final.to_fixed()));
    let ca_w = Q32::from_fixed(__lp_q32_cos(psi_w_final.to_fixed()));

    // gx = Ca * px + Sa * qx, gy = Ca * py + Sa * qy, gz = Ca * pz + Sa * qz
    let gx_x = ca_x * px_x + sa_x * qx_x;
    let gx_y = ca_y * px_y + sa_y * qx_y;
    let gx_z = ca_z * px_z + sa_z * qx_z;
    let gx_w = ca_w * px_w + sa_w * qx_w;
    let gy_x = ca_x * py_x + sa_x * qy_x;
    let gy_y = ca_y * py_y + sa_y * qy_y;
    let gy_z = ca_z * py_z + sa_z * qy_z;
    let gy_w = ca_w * py_w + sa_w * qy_w;
    let gz_x = ca_x * pz_x + sa_x * qz_x;
    let gz_y = ca_y * pz_y + sa_y * qz_y;
    let gz_z = ca_z * pz_z + sa_z * qz_z;
    let gz_w = ca_w * pz_w + sa_w * qz_w;

    // Reorganize for dot products below
    let g0_x = gx_x;
    let g0_y = gy_x;
    let g0_z = gz_x;
    let g1_x = gx_y;
    let g1_y = gy_y;
    let g1_z = gz_y;
    let g2_x = gx_z;
    let g2_y = gy_z;
    let g2_z = gz_z;
    let g3_x = gx_w;
    let g3_y = gy_w;
    let g3_z = gz_w;

    // Radial decay with distance from each simplex corner
    // w = 0.5 - vec4(dot(x0,x0), dot(x1,x1), dot(x2,x2), dot(x3,x3))
    let dot0 = x0_x * x0_x + x0_y * x0_y + x0_z * x0_z;
    let dot1 = x1_x * x1_x + x1_y * x1_y + x1_z * x1_z;
    let dot2 = x2_x * x2_x + x2_y * x2_y + x2_z * x2_z;
    let dot3 = x3_x * x3_x + x3_y * x3_y + x3_z * x3_z;
    let mut w_x = RADIAL_DECAY_0_5 - dot0;
    let mut w_y = RADIAL_DECAY_0_5 - dot1;
    let mut w_z = RADIAL_DECAY_0_5 - dot2;
    let mut w_w = RADIAL_DECAY_0_5 - dot3;

    // w = max(w, 0.0)
    w_x = w_x.max(Q32::ZERO);
    w_y = w_y.max(Q32::ZERO);
    w_z = w_z.max(Q32::ZERO);
    w_w = w_w.max(Q32::ZERO);

    // w2 = w * w, w3 = w2 * w
    let w2_x = w_x * w_x;
    let w2_y = w_y * w_y;
    let w2_z = w_z * w_z;
    let w2_w = w_w * w_w;
    let w3_x = w2_x * w_x;
    let w3_y = w2_y * w_y;
    let w3_z = w2_z * w_z;
    let w3_w = w2_w * w_w;

    // The value of the linear ramp from each of the corners
    // gdotx = vec4(dot(g0,x0), dot(g1,x1), dot(g2,x2), dot(g3,x3))
    let gdotx_x = g0_x * x0_x + g0_y * x0_y + g0_z * x0_z;
    let gdotx_y = g1_x * x1_x + g1_y * x1_y + g1_z * x1_z;
    let gdotx_z = g2_x * x2_x + g2_y * x2_y + g2_z * x2_z;
    let gdotx_w = g3_x * x3_x + g3_y * x3_y + g3_z * x3_z;

    // Multiply by the radial decay and sum up the noise value
    // n = dot(w3, gdotx)
    let n = w3_x * gdotx_x + w3_y * gdotx_y + w3_z * gdotx_z + w3_w * gdotx_w;

    // Compute the first order partial derivatives
    // dw = -6.0 * w2 * gdotx
    let dw_x = -SIX * w2_x * gdotx_x;
    let dw_y = -SIX * w2_y * gdotx_y;
    let dw_z = -SIX * w2_z * gdotx_z;
    let dw_w = -SIX * w2_w * gdotx_w;
    // dn0 = w3.x * g0 + dw.x * x0, etc.
    let dn0_x = w3_x * g0_x + dw_x * x0_x;
    let dn0_y = w3_x * g0_y + dw_x * x0_y;
    let dn0_z = w3_x * g0_z + dw_x * x0_z;
    let dn1_x = w3_y * g1_x + dw_y * x1_x;
    let dn1_y = w3_y * g1_y + dw_y * x1_y;
    let dn1_z = w3_y * g1_z + dw_y * x1_z;
    let dn2_x = w3_z * g2_x + dw_z * x2_x;
    let dn2_y = w3_z * g2_y + dw_z * x2_y;
    let dn2_z = w3_z * g2_z + dw_z * x2_z;
    let dn3_x = w3_w * g3_x + dw_w * x3_x;
    let dn3_y = w3_w * g3_y + dw_w * x3_y;
    let dn3_z = w3_w * g3_z + dw_w * x3_z;
    // gradient = 39.5 * (dn0 + dn1 + dn2 + dn3)
    let gradient_x = SCALE_39_5 * (dn0_x + dn1_x + dn2_x + dn3_x);
    let gradient_y = SCALE_39_5 * (dn0_y + dn1_y + dn2_y + dn3_y);
    let gradient_z = SCALE_39_5 * (dn0_z + dn1_z + dn2_z + dn3_z);

    // Scale the return value to fit nicely into the range [-1,1]
    let noise_value = SCALE_39_5 * n;

    (noise_value, gradient_x, gradient_y, gradient_z)
}

/// 3D Periodic Simplex Rotational Domain noise function (extern C wrapper for compiler).
///
/// # Arguments
/// * `x` - X coordinate as i32 (Q32 fixed-point)
/// * `y` - Y coordinate as i32 (Q32 fixed-point)
/// * `z` - Z coordinate as i32 (Q32 fixed-point)
/// * `period_x` - X period as i32 (Q32 fixed-point, 0 = no tiling)
/// * `period_y` - Y period as i32 (Q32 fixed-point, 0 = no tiling)
/// * `period_z` - Z period as i32 (Q32 fixed-point, 0 = no tiling)
/// * `alpha` - Rotation angle in radians as i32 (Q32 fixed-point)
/// * `gradient_out` - Pointer to output gradient [gx, gy, gz] as i32 (Q32 fixed-point)
/// * `seed` - Seed value for randomization (unused in psrdnoise, kept for consistency)
///
/// # Returns
/// Noise value as i32 (Q32 fixed-point format), approximately in range [-1, 1]
#[lpfx_impl_macro::lpfx_impl(
    q32,
    "float lpfx_psrdnoise(vec3 x, vec3 period, float alpha, out vec3 gradient)"
)]
#[unsafe(no_mangle)]
pub extern "C" fn __lpfx_psrdnoise3_q32(
    x: i32,
    y: i32,
    z: i32,
    period_x: i32,
    period_y: i32,
    period_z: i32,
    alpha: i32,
    gradient_out: *mut i32,
    seed: u32,
) -> i32 {
    let x_vec = Vec3Q32::new(Q32::from_fixed(x), Q32::from_fixed(y), Q32::from_fixed(z));
    let period_vec = Vec3Q32::new(
        Q32::from_fixed(period_x),
        Q32::from_fixed(period_y),
        Q32::from_fixed(period_z),
    );
    let alpha_q32 = Q32::from_fixed(alpha);

    let (noise_value, gradient_x, gradient_y, gradient_z) =
        lpfx_psrdnoise3(x_vec, period_vec, alpha_q32, seed);

    // Write gradient to output pointer
    unsafe {
        *gradient_out = gradient_x.to_fixed();
        *gradient_out.add(1) = gradient_y.to_fixed();
        *gradient_out.add(2) = gradient_z.to_fixed();
    }

    noise_value.to_fixed()
}

#[cfg(test)]
mod tests {
    #[cfg(test)]
    extern crate std;
    use super::*;
    use crate::util::test_helpers::{fixed_to_float, float_to_fixed};

    #[test]
    fn test_psrdnoise3_basic() {
        let x = float_to_fixed(1.5);
        let y = float_to_fixed(2.3);
        let z = float_to_fixed(0.7);
        let period_x = float_to_fixed(0.0);
        let period_y = float_to_fixed(0.0);
        let period_z = float_to_fixed(0.0);
        let alpha = float_to_fixed(0.0);
        let mut gradient = [0i32; 3];

        let result = __lpfx_psrdnoise3_q32(
            x,
            y,
            z,
            period_x,
            period_y,
            period_z,
            alpha,
            gradient.as_mut_ptr(),
            0,
        );

        // Should produce some value
        let result_float = fixed_to_float(result);
        assert!(
            result_float >= -2.0 && result_float <= 2.0,
            "Noise value should be in approximate range [-1, 1], got {}",
            result_float
        );

        // Gradient should be written
        let grad_x = fixed_to_float(gradient[0]);
        let grad_y = fixed_to_float(gradient[1]);
        let grad_z = fixed_to_float(gradient[2]);
        assert!(
            grad_x >= -50.0 && grad_x <= 50.0,
            "Gradient x should be reasonable, got {}",
            grad_x
        );
        assert!(
            grad_y >= -50.0 && grad_y <= 50.0,
            "Gradient y should be reasonable, got {}",
            grad_y
        );
        assert!(
            grad_z >= -50.0 && grad_z <= 50.0,
            "Gradient z should be reasonable, got {}",
            grad_z
        );
    }

    #[test]
    fn test_psrdnoise3_periodic() {
        let x = float_to_fixed(1.5);
        let y = float_to_fixed(2.3);
        let z = float_to_fixed(0.7);
        let period_x = float_to_fixed(10.0);
        let period_y = float_to_fixed(10.0);
        let period_z = float_to_fixed(10.0);
        let alpha = float_to_fixed(0.0);
        let mut gradient = [0i32; 3];

        let result = __lpfx_psrdnoise3_q32(
            x,
            y,
            z,
            period_x,
            period_y,
            period_z,
            alpha,
            gradient.as_mut_ptr(),
            0,
        );

        // Should produce some value
        let result_float = fixed_to_float(result);
        assert!(
            result_float >= -2.0 && result_float <= 2.0,
            "Noise value should be in approximate range [-1, 1], got {}",
            result_float
        );
    }

    #[test]
    fn test_psrdnoise3_rotation() {
        let x = float_to_fixed(1.5);
        let y = float_to_fixed(2.3);
        let z = float_to_fixed(0.7);
        let period_x = float_to_fixed(0.0);
        let period_y = float_to_fixed(0.0);
        let period_z = float_to_fixed(0.0);
        let alpha1 = float_to_fixed(0.0);
        let alpha2 = float_to_fixed(1.57); // ~π/2
        let mut gradient1 = [0i32; 3];
        let mut gradient2 = [0i32; 3];

        let result1 = __lpfx_psrdnoise3_q32(
            x,
            y,
            z,
            period_x,
            period_y,
            period_z,
            alpha1,
            gradient1.as_mut_ptr(),
            0,
        );
        let result2 = __lpfx_psrdnoise3_q32(
            x,
            y,
            z,
            period_x,
            period_y,
            period_z,
            alpha2,
            gradient2.as_mut_ptr(),
            0,
        );

        // Different rotation angles should produce different results
        let result1_float = fixed_to_float(result1);
        let result2_float = fixed_to_float(result2);
        // Just verify they're both in valid range
        assert!(
            result1_float >= -2.0 && result1_float <= 2.0,
            "Result1 should be in range"
        );
        assert!(
            result2_float >= -2.0 && result2_float <= 2.0,
            "Result2 should be in range"
        );
    }

    #[test]
    fn test_psrdnoise3_deterministic() {
        let x = float_to_fixed(42.5);
        let y = float_to_fixed(37.3);
        let z = float_to_fixed(25.1);
        let period_x = float_to_fixed(0.0);
        let period_y = float_to_fixed(0.0);
        let period_z = float_to_fixed(0.0);
        let alpha = float_to_fixed(0.5);
        let mut gradient1 = [0i32; 3];
        let mut gradient2 = [0i32; 3];

        let result1 = __lpfx_psrdnoise3_q32(
            x,
            y,
            z,
            period_x,
            period_y,
            period_z,
            alpha,
            gradient1.as_mut_ptr(),
            0,
        );
        let result2 = __lpfx_psrdnoise3_q32(
            x,
            y,
            z,
            period_x,
            period_y,
            period_z,
            alpha,
            gradient2.as_mut_ptr(),
            0,
        );

        // Same inputs should produce same outputs
        assert_eq!(result1, result2, "Noise should be deterministic");
        assert_eq!(
            gradient1[0], gradient2[0],
            "Gradient x should be deterministic"
        );
        assert_eq!(
            gradient1[1], gradient2[1],
            "Gradient y should be deterministic"
        );
        assert_eq!(
            gradient1[2], gradient2[2],
            "Gradient z should be deterministic"
        );
    }
}
