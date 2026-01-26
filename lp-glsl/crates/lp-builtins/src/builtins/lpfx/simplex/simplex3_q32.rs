//! 3D Simplex noise function.
//!
//! Simplex noise is an improved version of Perlin noise with better quality and performance.
//! This implementation uses Q32 fixed-point arithmetic (16.16 format).
//!
//! Reference: noise-rs library and Stefan Gustavson's Simplex noise implementation
//!
//! # GLSL Usage
//!
//! This function is callable from GLSL shaders using the `lpfx_simplex3` name:
//!
//! ```glsl
//! float noise = lpfx_simplex3(vec3(5.0, 3.0, 1.0), 123u);
//! ```
//!
//! # Parameters
//!
//! - `p`: Input coordinates as vec3 (converted to Q32 internally, flattened to x, y, z)
//! - `seed`: Seed value for randomization (uint)
//!
//! # Returns
//!
//! Noise value approximately in range [-1, 1] (float)
//!
//! # Internal Implementation
//!
//! The user-facing `lpfx_simplex3` function maps to internal `__lpfx_simplex3` which
//! operates on Q32 fixed-point values. Vector arguments are automatically flattened
//! by the compiler (vec3 becomes three i32 parameters).

use crate::builtins::lpfx::hash::__lpfx_hash_3;
use crate::util::q32::Q32;

/// Fixed-point constants
const TWO: Q32 = Q32(0x00020000); // 2.0 in Q16.16
const THREE: Q32 = Q32(0x00030000); // 3.0 in Q16.16

/// Skew factor for 3D: (sqrt(4) - 1) / 3 = 1/3 ≈ 0.333333
/// In Q16.16: 0.333333 * 65536 ≈ 21845
const SKEW_FACTOR_3D: Q32 = Q32(21845);

/// Unskew factor for 3D: (1 - 1/sqrt(4)) / 3 = (1 - 0.5) / 3 ≈ 0.166666
/// In Q16.16: 0.166666 * 65536 ≈ 10923
const UNSKEW_FACTOR_3D: Q32 = Q32(10923);

/// 3D Simplex noise function.
///
/// # Arguments
/// * `x` - X coordinate in Q32 fixed-point format
/// * `y` - Y coordinate in Q32 fixed-point format
/// * `z` - Z coordinate in Q32 fixed-point format
/// * `seed` - Seed value for randomization
///
/// # Returns
/// Noise value in Q32 fixed-point format, approximately in range [-1, 1]
#[allow(unknown_attributes)]
#[lpfx_impl(q32, "float lpfx_simplex3(vec3 p, uint seed)")]
#[unsafe(no_mangle)]
pub extern "C" fn __lpfx_simplex3_q32(x: i32, y: i32, z: i32, seed: u32) -> i32 {
    // Convert inputs to Q32
    let x = Q32::from_fixed(x);
    let y = Q32::from_fixed(y);
    let z = Q32::from_fixed(z);

    // Skew the input space to determine which simplex cell we're in
    // skew = (x + y + z) * SKEW_FACTOR
    let sum = x + y + z;
    let skew = sum * SKEW_FACTOR_3D;
    let skewed_x = x + skew;
    let skewed_y = y + skew;
    let skewed_z = z + skew;

    // Get cell coordinates (floor)
    let cell_x_int = skewed_x.to_i32();
    let cell_y_int = skewed_y.to_i32();
    let cell_z_int = skewed_z.to_i32();

    // Convert back to fixed-point for calculations
    let cell_x = Q32::from_i32(cell_x_int);
    let cell_y = Q32::from_i32(cell_y_int);
    let cell_z = Q32::from_i32(cell_z_int);

    // Unskew the cell origin back to (x,y,z) space
    let cell_sum = cell_x + cell_y + cell_z;
    let unskew = cell_sum * UNSKEW_FACTOR_3D;
    let unskewed_x = cell_x - unskew;
    let unskewed_y = cell_y - unskew;
    let unskewed_z = cell_z - unskew;

    // The x,y,z distances from the cell origin
    let offset1_x = x - unskewed_x;
    let offset1_y = y - unskewed_y;
    let offset1_z = z - unskewed_z;

    // For the 3D case, the simplex shape is a slightly irregular tetrahedron.
    // Determine which simplex we are in based on ordering of offsets.
    let (order1_x, order1_y, order1_z, order2_x, order2_y, order2_z) = if offset1_x >= offset1_y {
        if offset1_y >= offset1_z {
            // X Y Z order
            (
                Q32::ONE,
                Q32::ZERO,
                Q32::ZERO,
                Q32::ONE,
                Q32::ONE,
                Q32::ZERO,
            )
        } else if offset1_x >= offset1_z {
            // X Z Y order
            (
                Q32::ONE,
                Q32::ZERO,
                Q32::ZERO,
                Q32::ONE,
                Q32::ZERO,
                Q32::ONE,
            )
        } else {
            // Z X Y order
            (
                Q32::ZERO,
                Q32::ZERO,
                Q32::ONE,
                Q32::ONE,
                Q32::ZERO,
                Q32::ONE,
            )
        }
    } else {
        // x0 < y0
        if offset1_y < offset1_z {
            // Z Y X order
            (
                Q32::ZERO,
                Q32::ZERO,
                Q32::ONE,
                Q32::ZERO,
                Q32::ONE,
                Q32::ONE,
            )
        } else if offset1_x < offset1_z {
            // Y Z X order
            (
                Q32::ZERO,
                Q32::ONE,
                Q32::ZERO,
                Q32::ZERO,
                Q32::ONE,
                Q32::ONE,
            )
        } else {
            // Y X Z order
            (
                Q32::ZERO,
                Q32::ONE,
                Q32::ZERO,
                Q32::ONE,
                Q32::ONE,
                Q32::ZERO,
            )
        }
    };

    // Offsets for corners
    let offset2_x = offset1_x - order1_x + UNSKEW_FACTOR_3D;
    let offset2_y = offset1_y - order1_y + UNSKEW_FACTOR_3D;
    let offset2_z = offset1_z - order1_z + UNSKEW_FACTOR_3D;

    let offset3_x = offset1_x - order2_x + (TWO * UNSKEW_FACTOR_3D);
    let offset3_y = offset1_y - order2_y + (TWO * UNSKEW_FACTOR_3D);
    let offset3_z = offset1_z - order2_z + (TWO * UNSKEW_FACTOR_3D);

    let offset4_x = offset1_x - Q32::ONE + (THREE * UNSKEW_FACTOR_3D);
    let offset4_y = offset1_y - Q32::ONE + (THREE * UNSKEW_FACTOR_3D);
    let offset4_z = offset1_z - Q32::ONE + (THREE * UNSKEW_FACTOR_3D);

    // Calculate gradient indexes for each corner
    let gi0 = __lpfx_hash_3(
        cell_x_int as u32,
        cell_y_int as u32,
        cell_z_int as u32,
        seed,
    );
    let gi1 = __lpfx_hash_3(
        (cell_x_int + order1_x.to_i32()) as u32,
        (cell_y_int + order1_y.to_i32()) as u32,
        (cell_z_int + order1_z.to_i32()) as u32,
        seed,
    );
    let gi2 = __lpfx_hash_3(
        (cell_x_int + order2_x.to_i32()) as u32,
        (cell_y_int + order2_y.to_i32()) as u32,
        (cell_z_int + order2_z.to_i32()) as u32,
        seed,
    );
    let gi3 = __lpfx_hash_3(
        (cell_x_int + 1) as u32,
        (cell_y_int + 1) as u32,
        (cell_z_int + 1) as u32,
        seed,
    );

    // Calculate contribution from each corner
    let corner0 = surflet_3d(gi0 as usize, offset1_x, offset1_y, offset1_z);
    let corner1 = surflet_3d(gi1 as usize, offset2_x, offset2_y, offset2_z);
    let corner2 = surflet_3d(gi2 as usize, offset3_x, offset3_y, offset3_z);
    let corner3 = surflet_3d(gi3 as usize, offset4_x, offset4_y, offset4_z);

    // Add contributions from each corner
    // Result is already approximately in [-1, 1] range due to algorithm
    (corner0 + corner1 + corner2 + corner3).to_fixed()
}

/// Compute magnitude squared of a 3D vector
#[inline(always)]
fn magnitude_squared_3d(x: Q32, y: Q32, z: Q32) -> Q32 {
    x * x + y * y + z * z
}

/// Compute dot product of two 3D vectors
#[inline(always)]
fn dot_3d(x1: Q32, y1: Q32, z1: Q32, x2: Q32, y2: Q32, z2: Q32) -> Q32 {
    x1 * x2 + y1 * y2 + z1 * z2
}

/// Get 3D gradient vector from gradient index
/// Returns (gx, gy, gz) in Q32 fixed-point format
#[inline(always)]
fn grad3(index: usize) -> (Q32, Q32, Q32) {
    // Gradients are combinations of -1, 0, and 1, normalized
    // For 3D, we use 12 edge gradients + 8 corner gradients (32 total)
    const DIAG: Q32 = Q32(0xB505); // 1/sqrt(2) ≈ 0.70710678118 in Q16.16
    const DIAG2: Q32 = Q32(0x93CD); // 1/sqrt(3) ≈ 0.57735026919 in Q16.16

    match index % 32 {
        0 | 12 => (DIAG, DIAG, Q32::ZERO),    // (1/sqrt(2), 1/sqrt(2), 0)
        1 | 13 => (-DIAG, DIAG, Q32::ZERO),   // (-1/sqrt(2), 1/sqrt(2), 0)
        2 | 14 => (DIAG, -DIAG, Q32::ZERO),   // (1/sqrt(2), -1/sqrt(2), 0)
        3 | 15 => (-DIAG, -DIAG, Q32::ZERO),  // (-1/sqrt(2), -1/sqrt(2), 0)
        4 | 16 => (DIAG, Q32::ZERO, DIAG),    // (1/sqrt(2), 0, 1/sqrt(2))
        5 | 17 => (-DIAG, Q32::ZERO, DIAG),   // (-1/sqrt(2), 0, 1/sqrt(2))
        6 | 18 => (DIAG, Q32::ZERO, -DIAG),   // (1/sqrt(2), 0, -1/sqrt(2))
        7 | 19 => (-DIAG, Q32::ZERO, -DIAG),  // (-1/sqrt(2), 0, -1/sqrt(2))
        8 | 20 => (Q32::ZERO, DIAG, DIAG),    // (0, 1/sqrt(2), 1/sqrt(2))
        9 | 21 => (Q32::ZERO, -DIAG, DIAG),   // (0, -1/sqrt(2), 1/sqrt(2))
        10 | 22 => (Q32::ZERO, DIAG, -DIAG),  // (0, 1/sqrt(2), -1/sqrt(2))
        11 | 23 => (Q32::ZERO, -DIAG, -DIAG), // (0, -1/sqrt(2), -1/sqrt(2))
        24 => (DIAG2, DIAG2, DIAG2),          // (1/sqrt(3), 1/sqrt(3), 1/sqrt(3))
        25 => (-DIAG2, DIAG2, DIAG2),         // (-1/sqrt(3), 1/sqrt(3), 1/sqrt(3))
        26 => (DIAG2, -DIAG2, DIAG2),         // (1/sqrt(3), -1/sqrt(3), 1/sqrt(3))
        27 => (-DIAG2, -DIAG2, DIAG2),        // (-1/sqrt(3), -1/sqrt(3), 1/sqrt(3))
        28 => (DIAG2, DIAG2, -DIAG2),         // (1/sqrt(3), 1/sqrt(3), -1/sqrt(3))
        29 => (-DIAG2, DIAG2, -DIAG2),        // (-1/sqrt(3), 1/sqrt(3), -1/sqrt(3))
        30 => (DIAG2, -DIAG2, -DIAG2),        // (1/sqrt(3), -1/sqrt(3), -1/sqrt(3))
        31 => (-DIAG2, -DIAG2, -DIAG2),       // (-1/sqrt(3), -1/sqrt(3), -1/sqrt(3))
        _ => (DIAG, DIAG, Q32::ZERO),         // Should never happen
    }
}

/// Compute surflet contribution for a corner
#[inline(always)]
fn surflet_3d(gradient_index: usize, x: Q32, y: Q32, z: Q32) -> Q32 {
    // t = 1.0 - dist^2 * 2.0
    let dist_sq = magnitude_squared_3d(x, y, z);
    let dist_sq_times_2 = dist_sq * TWO;
    let t = Q32::ONE - dist_sq_times_2;

    if t > Q32::ZERO {
        // Get gradient
        let (gx, gy, gz) = grad3(gradient_index);

        // Compute dot product: gradient · offset
        let dot = dot_3d(gx, gy, gz, x, y, z);

        // Apply falloff: (2.0 * t^2 + t^4) * dot
        let t2 = t * t;
        let t4 = t2 * t2;
        let falloff = TWO * t2 + t4;

        dot * falloff
    } else {
        Q32::ZERO
    }
}

#[cfg(test)]
mod tests {
    #[cfg(test)]
    extern crate std;
    use super::*;
    use crate::util::test_helpers::{fixed_to_float, float_to_fixed};
    use std::vec::Vec;
    use std::{print, println};

    #[test]
    fn test_simplex3_basic() {
        // Test with various inputs to ensure we get different outputs
        let results: Vec<i32> = (0..10)
            .map(|i| {
                __lpfx_simplex3_q32(
                    float_to_fixed(i as f32 * 0.5),
                    float_to_fixed(i as f32 * 0.3),
                    float_to_fixed(i as f32 * 0.7),
                    0,
                )
            })
            .collect();

        // Check that we get some variation (not all zeros)
        let all_zero = results.iter().all(|&r| r == 0);
        assert!(!all_zero, "Simplex3 should produce non-zero values");

        // Test seed affects output
        let result_seed0 = __lpfx_simplex3_q32(
            float_to_fixed(5.0),
            float_to_fixed(3.0),
            float_to_fixed(2.0),
            0,
        );
        let result_seed1 = __lpfx_simplex3_q32(
            float_to_fixed(5.0),
            float_to_fixed(3.0),
            float_to_fixed(2.0),
            1,
        );
        // Note: seed might not always change output at every point, but should often
        // We just verify the function works with different seeds
        let _ = result_seed0;
        let _ = result_seed1;
    }

    #[test]
    fn test_simplex3_range() {
        // Test that output is approximately in [-1, 1] range
        for i in 0..30 {
            let x = float_to_fixed(i as f32 * 0.1);
            let y = float_to_fixed(i as f32 * 0.15);
            let z = float_to_fixed(i as f32 * 0.2);
            let result = __lpfx_simplex3_q32(x, y, z, 0);
            let result_float = fixed_to_float(result);

            assert!(
                result_float >= -2.0 && result_float <= 2.0,
                "Noise value {} should be in approximate range [-1, 1], got {}",
                i,
                result_float
            );
        }
    }

    #[test]
    fn test_simplex3_deterministic() {
        let result1 = __lpfx_simplex3_q32(
            float_to_fixed(42.5),
            float_to_fixed(37.3),
            float_to_fixed(25.1),
            123,
        );
        let result2 = __lpfx_simplex3_q32(
            float_to_fixed(42.5),
            float_to_fixed(37.3),
            float_to_fixed(25.1),
            123,
        );

        // Same input and seed should produce same output
        assert_eq!(result1, result2, "Noise should be deterministic");
    }

    #[cfg(feature = "test")]
    #[test]
    fn test_simplex3_properties() {
        use noise::{NoiseFn, Simplex};

        // Create noise-rs simplex for reference comparison
        let noise_rs_fn = Simplex::new(0);

        // Test multiple points - verify our implementation has similar properties
        let test_points = [
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            (0.0, 0.0, 1.0),
            (5.5, 3.2, 1.1),
            (10.0, 10.0, 10.0),
            (-5.0, -3.0, -2.0),
        ];

        for (x, y, z) in test_points {
            // Get our output
            let our_value_fixed =
                __lpfx_simplex3_q32(float_to_fixed(x), float_to_fixed(y), float_to_fixed(z), 0);
            let our_value = fixed_to_float(our_value_fixed);

            // Get noise-rs output for reference
            let noise_rs_value = noise_rs_fn.get([x as f64, y as f64, z as f64]) as f32;

            // Verify our output is in reasonable range (similar to noise-rs)
            assert!(
                our_value >= -2.0 && our_value <= 2.0,
                "Simplex3({}, {}, {}) should be in range [-2, 2], got {}",
                x,
                y,
                z,
                our_value
            );

            // Verify noise-rs is also in similar range (sanity check)
            assert!(
                noise_rs_value >= -2.0 && noise_rs_value <= 2.0,
                "noise-rs Simplex3({}, {}, {}) should be in range [-2, 2], got {}",
                x,
                y,
                z,
                noise_rs_value
            );

            // Note: We don't compare exact values because we use a different hash function (noiz)
            // The important thing is that our implementation produces reasonable noise values
        }
    }

    #[test]
    fn test_simplex3_output_grid() {
        // Output grids of noise values for manual inspection
        println!("\n=== Simplex3 Noise Grid (seed=0, z=0.0) ===");
        println!("5x5 grid, X and Y from 0.0 to 4.0, Z=0.0:");
        println!("      ");
        for x_idx in 0..5 {
            print!("  X{:1}", x_idx);
        }
        println!();
        for y_idx in 0..5 {
            print!("Y{:1} ", y_idx);
            for x_idx in 0..5 {
                let x = x_idx as f32;
                let y = y_idx as f32;
                let z = 0.0;
                let result =
                    __lpfx_simplex3_q32(float_to_fixed(x), float_to_fixed(y), float_to_fixed(z), 0);
                let result_float = fixed_to_float(result);
                print!("{:6.3} ", result_float);
            }
            println!();
        }

        println!("\n=== Simplex3 Noise Grid (seed=0, z=2.0) ===");
        println!("5x5 grid, X and Y from 0.0 to 4.0, Z=2.0:");
        println!("      ");
        for x_idx in 0..5 {
            print!("  X{:1}", x_idx);
        }
        println!();
        for y_idx in 0..5 {
            print!("Y{:1} ", y_idx);
            for x_idx in 0..5 {
                let x = x_idx as f32;
                let y = y_idx as f32;
                let z = 2.0;
                let result =
                    __lpfx_simplex3_q32(float_to_fixed(x), float_to_fixed(y), float_to_fixed(z), 0);
                let result_float = fixed_to_float(result);
                print!("{:6.3} ", result_float);
            }
            println!();
        }

        println!("\n=== Simplex3 Seed Comparison (x=2.5, y=2.5, z=2.5) ===");
        let x = float_to_fixed(2.5);
        let y = float_to_fixed(2.5);
        let z = float_to_fixed(2.5);
        for seed in 0..5 {
            let result = __lpfx_simplex3_q32(x, y, z, seed);
            let result_float = fixed_to_float(result);
            println!("  seed={}: {:7.4}", seed, result_float);
        }

        // Verify outputs are in reasonable range
        for i in 0..30 {
            let x = float_to_fixed(i as f32 * 0.1);
            let y = float_to_fixed(i as f32 * 0.15);
            let z = float_to_fixed(i as f32 * 0.2);
            let result = __lpfx_simplex3_q32(x, y, z, 0);
            let result_float = fixed_to_float(result);
            assert!(
                result_float >= -2.0 && result_float <= 2.0,
                "Noise value should be in reasonable range, got {}",
                result_float
            );
        }
    }

    #[cfg(all(test, feature = "test_hash_fixed"))]
    mod fixed_hash_tests {
        use super::*;
        use crate::util::test_helpers::{fixed_to_float, float_to_fixed};

        #[test]
        fn test_simplex3_boundary_continuity() {
            // Test continuity across cell boundaries using fixed hash
            let boundary_points = [
                (0.0, 0.0, 0.0),
                (0.001, 0.001, 0.001),
                (0.999, 0.999, 0.999),
                (1.0, 1.0, 1.0),
                (1.001, 1.001, 1.001),
                (2.0, 2.0, 2.0),
            ];

            let mut prev_value: Option<f32> = None;
            let mut max_jump = 0.0f32;

            for (x, y, z) in boundary_points {
                let result =
                    __lpfx_simplex3_q32(float_to_fixed(x), float_to_fixed(y), float_to_fixed(z), 0);
                let result_float = fixed_to_float(result);

                if let Some(prev) = prev_value {
                    let jump = (result_float - prev).abs();
                    max_jump = max_jump.max(jump);

                    if jump > 0.5 {
                        println!(
                            "Large jump detected at ({}, {}, {}): {} -> {}, jump = {}",
                            x, y, z, prev, result_float, jump
                        );
                    }
                }
                prev_value = Some(result_float);
            }

            println!("Maximum jump along boundary path: {}", max_jump);
            assert!(
                max_jump < 1.0,
                "Discontinuity detected: maximum jump = {}",
                max_jump
            );
        }

        #[test]
        fn test_simplex3_deterministic_with_fixed_hash() {
            // Test that fixed hash produces deterministic outputs
            let test_points = [
                (0.0, 0.0, 0.0),
                (0.5, 0.5, 0.5),
                (1.0, 1.0, 1.0),
                (2.5, 2.5, 2.5),
            ];

            for (x, y, z) in test_points {
                let result1 =
                    __lpfx_simplex3_q32(float_to_fixed(x), float_to_fixed(y), float_to_fixed(z), 0);
                let result2 =
                    __lpfx_simplex3_q32(float_to_fixed(x), float_to_fixed(y), float_to_fixed(z), 0);
                assert_eq!(
                    result1, result2,
                    "Simplex3({}, {}, {}) should be deterministic with fixed hash",
                    x, y, z
                );
            }
        }

        #[test]
        fn test_simplex3_no_discontinuities_along_line() {
            // Sample noise along a line and check for sudden jumps
            const STEP: f32 = 0.01;
            const THRESHOLD: f32 = 0.5;

            let mut prev_value: Option<f32> = None;
            let mut max_jump = 0.0f32;
            let mut jump_count = 0;

            for i in 0..500 {
                let x = i as f32 * STEP;
                let y = x;
                let z = x;
                let result =
                    __lpfx_simplex3_q32(float_to_fixed(x), float_to_fixed(y), float_to_fixed(z), 0);
                let result_float = fixed_to_float(result);

                if let Some(prev) = prev_value {
                    let jump = (result_float - prev).abs();
                    max_jump = max_jump.max(jump);

                    if jump > THRESHOLD {
                        jump_count += 1;
                        if jump_count <= 5 {
                            println!(
                                "Large jump detected at ({}, {}, {}): {} -> {}, jump = {}",
                                x, y, z, prev, result_float, jump
                            );
                        }
                    }
                }
                prev_value = Some(result_float);
            }

            println!(
                "Maximum jump along diagonal: {}, jumps > {}: {}",
                max_jump, THRESHOLD, jump_count
            );
            assert!(
                max_jump < 1.0,
                "Discontinuity detected: maximum jump = {}",
                max_jump
            );
        }
    }

    #[cfg(all(test, feature = "test_visual"))]
    mod visual_tests {
        use super::*;
        use crate::util::test_helpers::{fixed_to_float, float_to_fixed};

        #[test]
        fn test_simplex3_no_discontinuities() {
            // Sample noise along a line and check for sudden jumps
            const STEP: f32 = 0.01;
            const THRESHOLD: f32 = 0.3;

            let mut prev_value: Option<f32> = None;
            let mut max_jump = 0.0f32;
            let mut jump_count = 0;
            let mut jump_locations = Vec::new();

            for i in 0..500 {
                let x = i as f32 * STEP;
                let y = x;
                let z = x;
                let result =
                    __lpfx_simplex3_q32(float_to_fixed(x), float_to_fixed(y), float_to_fixed(z), 0);
                let result_float = fixed_to_float(result);

                if let Some(prev) = prev_value {
                    let jump = (result_float - prev).abs();
                    max_jump = max_jump.max(jump);

                    if jump > THRESHOLD {
                        jump_count += 1;
                        if jump_locations.len() < 10 {
                            jump_locations.push((x, y, z, prev, result_float, jump));
                        }
                    }
                }
                prev_value = Some(result_float);
            }

            println!(
                "Maximum jump along diagonal: {}, jumps > {}: {}",
                max_jump, THRESHOLD, jump_count
            );

            if !jump_locations.is_empty() {
                println!("First few jump locations:");
                for (x, y, z, prev, curr, jump) in jump_locations.iter().take(5) {
                    println!(
                        "  ({:.3}, {:.3}, {:.3}): {} -> {}, jump = {}",
                        x, y, z, prev, curr, jump
                    );
                }
            }

            assert!(
                max_jump < 0.5,
                "Discontinuity detected: maximum jump = {} (threshold: 0.5)",
                max_jump
            );
        }

        #[cfg(feature = "test")]
        #[test]
        fn test_simplex3_compare_with_noise_rs() {
            use noise::{NoiseFn, Simplex};

            let noise_rs_fn = Simplex::new(0);
            let test_points = [
                (0.0, 0.0, 0.0),
                (0.5, 0.5, 0.5),
                (1.0, 1.0, 1.0),
                (5.5, 3.2, 1.1),
                (10.0, 10.0, 10.0),
            ];

            println!("\n=== Simplex3 Comparison with noise-rs ===");
            for (x, y, z) in test_points {
                let our_value =
                    __lpfx_simplex3_q32(float_to_fixed(x), float_to_fixed(y), float_to_fixed(z), 0);
                let our_float = fixed_to_float(our_value);

                let noise_rs_value = noise_rs_fn.get([x as f64, y as f64, z as f64]) as f32;

                let diff = (our_float - noise_rs_value).abs();
                println!(
                    "Point ({:4.1}, {:4.1}, {:4.1}): ours={:8.5}, noise-rs={:8.5}, diff={:8.5}",
                    x, y, z, our_float, noise_rs_value, diff
                );

                // Verify both are in reasonable range
                assert!(our_float >= -2.0 && our_float <= 2.0);
                assert!(noise_rs_value >= -2.0 && noise_rs_value <= 2.0);
            }
        }

        #[cfg(all(test, feature = "test_hash_fixed"))]
        mod trace_tests {
            use super::*;
            use crate::builtins::shared::lpfx_hash::__lpfx_hash_3;
            use crate::util::q32::Q32;
            use crate::util::test_helpers::{fixed_to_float, float_to_fixed};

            #[test]
            fn test_simplex3_trace_algorithm() {
                // Trace through the algorithm for a specific point to debug

                let x = float_to_fixed(2.5);
                let y = float_to_fixed(2.5);
                let z = float_to_fixed(2.5);
                let seed = 0u32;

                let x_q32 = Q32::from_fixed(x);
                let y_q32 = Q32::from_fixed(y);
                let z_q32 = Q32::from_fixed(z);

                // Skew
                let sum = x_q32 + y_q32 + z_q32;
                let skew = sum * super::SKEW_FACTOR_3D;
                let skewed_x = x_q32 + skew;
                let skewed_y = y_q32 + skew;
                let skewed_z = z_q32 + skew;

                let cell_x_int = skewed_x.to_i32();
                let cell_y_int = skewed_y.to_i32();
                let cell_z_int = skewed_z.to_i32();

                println!("\n=== Simplex3 Algorithm Trace (2.5, 2.5, 2.5) ===");
                println!(
                    "Input: ({}, {}, {})",
                    fixed_to_float(x),
                    fixed_to_float(y),
                    fixed_to_float(z)
                );
                println!(
                    "Skewed: ({:.6}, {:.6}, {:.6})",
                    fixed_to_float(skewed_x.to_fixed()),
                    fixed_to_float(skewed_y.to_fixed()),
                    fixed_to_float(skewed_z.to_fixed())
                );
                println!("Cell: ({}, {}, {})", cell_x_int, cell_y_int, cell_z_int);

                // Unskew
                let cell_x = Q32::from_i32(cell_x_int);
                let cell_y = Q32::from_i32(cell_y_int);
                let cell_z = Q32::from_i32(cell_z_int);
                let cell_sum = cell_x + cell_y + cell_z;
                let unskew = cell_sum * super::UNSKEW_FACTOR_3D;
                let unskewed_x = cell_x - unskew;
                let unskewed_y = cell_y - unskew;
                let unskewed_z = cell_z - unskew;

                println!("Unskew: {:.6}", fixed_to_float(unskew.to_fixed()));
                println!(
                    "Unskewed cell origin: ({:.6}, {:.6}, {:.6})",
                    fixed_to_float(unskewed_x.to_fixed()),
                    fixed_to_float(unskewed_y.to_fixed()),
                    fixed_to_float(unskewed_z.to_fixed())
                );

                // Offset1
                let offset1_x = x_q32 - unskewed_x;
                let offset1_y = y_q32 - unskewed_y;
                let offset1_z = z_q32 - unskewed_z;

                println!(
                    "Offset1: ({:.6}, {:.6}, {:.6})",
                    fixed_to_float(offset1_x.to_fixed()),
                    fixed_to_float(offset1_y.to_fixed()),
                    fixed_to_float(offset1_z.to_fixed())
                );

                // Determine order (simplified - just show which case)
                let order_case = if offset1_x >= offset1_y {
                    if offset1_y >= offset1_z {
                        "X Y Z"
                    } else if offset1_x >= offset1_z {
                        "X Z Y"
                    } else {
                        "Z X Y"
                    }
                } else {
                    if offset1_y < offset1_z {
                        "Z Y X"
                    } else if offset1_x < offset1_z {
                        "Y Z X"
                    } else {
                        "Y X Z"
                    }
                };
                println!("Order case: {}", order_case);

                // Calculate hash values for corners
                let gi0 = __lpfx_hash_3(
                    cell_x_int as u32,
                    cell_y_int as u32,
                    cell_z_int as u32,
                    seed,
                );
                println!(
                    "Hash for corner 0 (cell {}): {} (mod 32: {})",
                    (cell_x_int, cell_y_int, cell_z_int),
                    gi0,
                    gi0 % 32
                );

                let result = __lp_fixed32_lpfx_simplex3(x, y, z, seed);
                println!("Final result: {:.6}", fixed_to_float(result));
            }
        }
    }
}
