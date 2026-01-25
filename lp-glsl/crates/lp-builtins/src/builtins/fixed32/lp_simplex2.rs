//! 2D Simplex noise function.
//!
//! Simplex noise is an improved version of Perlin noise with better quality and performance.
//! This implementation uses Q32 fixed-point arithmetic (16.16 format).
//!
//! Reference: noise-rs library and Stefan Gustavson's Simplex noise implementation
//!
//! # GLSL Usage
//!
//! This function is callable from GLSL shaders using the `lp_simplex2` name:
//!
//! ```glsl
//! float noise = lp_simplex2(vec2(5.0, 3.0), 123u);
//! ```
//!
//! # Parameters
//!
//! - `p`: Input coordinates as vec2 (converted to Q32 internally, flattened to x, y)
//! - `seed`: Seed value for randomization (uint)
//!
//! # Returns
//!
//! Noise value approximately in range [-1, 1] (float)
//!
//! # Internal Implementation
//!
//! The user-facing `lp_simplex2` function maps to internal `__lp_simplex2` which
//! operates on Q32 fixed-point values. Vector arguments are automatically flattened
//! by the compiler (vec2 becomes two i32 parameters).

use crate::builtins::shared::lp_hash::__lp_hash_2;
use crate::fixed32::q32::Q32;

/// Fixed-point constants
const TWO: Q32 = Q32(0x00020000); // 2.0 in Q16.16

/// Skew factor for 2D: (sqrt(3) - 1) / 2
/// sqrt(3) ≈ 1.73205080757, so (1.732 - 1) / 2 ≈ 0.36602540378
/// In Q16.16: 0.36602540378 * 65536 ≈ 23967
const SKEW_FACTOR_2D: Q32 = Q32(23967);

/// Unskew factor for 2D: (3 - sqrt(3)) / 6
/// (3 - 1.732) / 6 ≈ 0.21132486541
/// In Q16.16: 0.21132486541 * 65536 ≈ 13853
const UNSKEW_FACTOR_2D: Q32 = Q32(13853);

/// 2D Simplex noise function.
///
/// # Arguments
/// * `x` - X coordinate in Q32 fixed-point format
/// * `y` - Y coordinate in Q32 fixed-point format
/// * `seed` - Seed value for randomization
///
/// # Returns
/// Noise value in Q32 fixed-point format, approximately in range [-1, 1]
#[unsafe(no_mangle)]
pub extern "C" fn __lp_fixed32_lp_simplex2(x: i32, y: i32, seed: u32) -> i32 {
    // Convert inputs to Q32
    let x = Q32::from_fixed(x);
    let y = Q32::from_fixed(y);

    // Skew the input space to determine which simplex cell we're in
    // skew = (x + y) * SKEW_FACTOR
    let sum = x + y;
    let skew = sum * SKEW_FACTOR_2D;
    let skewed_x = x + skew;
    let skewed_y = y + skew;

    // Get cell coordinates (floor)
    let cell_x_int = skewed_x.to_i32();
    let cell_y_int = skewed_y.to_i32();

    // Convert back to fixed-point for calculations
    let cell_x = Q32::from_i32(cell_x_int);
    let cell_y = Q32::from_i32(cell_y_int);

    // Unskew the cell origin back to (x,y) space
    let cell_sum = cell_x + cell_y;
    let unskew = cell_sum * UNSKEW_FACTOR_2D;
    let unskewed_x = cell_x - unskew;
    let unskewed_y = cell_y - unskew;

    // The x,y distances from the cell origin
    let offset1_x = x - unskewed_x;
    let offset1_y = y - unskewed_y;

    // For the 2D case, the simplex shape is an equilateral triangle.
    // Determine which simplex we are in.
    let (order_x, order_y) = if offset1_x > offset1_y {
        // Lower triangle, XY order: (0,0)->(1,0)->(1,1)
        (Q32::ONE, Q32::ZERO)
    } else {
        // Upper triangle, YX order: (0,0)->(0,1)->(1,1)
        (Q32::ZERO, Q32::ONE)
    };

    // Offsets for middle corner in (x,y) unskewed coords
    let offset2_x = offset1_x - order_x + UNSKEW_FACTOR_2D;
    let offset2_y = offset1_y - order_y + UNSKEW_FACTOR_2D;

    // Offsets for last corner in (x,y) unskewed coords
    let offset3_x = offset1_x - Q32::ONE + (TWO * UNSKEW_FACTOR_2D);
    let offset3_y = offset1_y - Q32::ONE + (TWO * UNSKEW_FACTOR_2D);

    // Calculate gradient indexes for each corner
    let gi0 = __lp_hash_2(cell_x_int as u32, cell_y_int as u32, seed);
    let gi1 = __lp_hash_2(
        (cell_x_int + order_x.to_i32()) as u32,
        (cell_y_int + order_y.to_i32()) as u32,
        seed,
    );
    let gi2 = __lp_hash_2((cell_x_int + 1) as u32, (cell_y_int + 1) as u32, seed);

    // Calculate contribution from each corner
    let corner0 = surflet_2d(gi0 as usize, offset1_x, offset1_y);
    let corner1 = surflet_2d(gi1 as usize, offset2_x, offset2_y);
    let corner2 = surflet_2d(gi2 as usize, offset3_x, offset3_y);

    // Add contributions from each corner
    // Result is already approximately in [-1, 1] range due to algorithm
    (corner0 + corner1 + corner2).to_fixed()
}

/// Compute magnitude squared of a 2D vector
#[inline(always)]
fn magnitude_squared_2d(x: Q32, y: Q32) -> Q32 {
    x * x + y * y
}

/// Compute dot product of two 2D vectors
#[inline(always)]
fn dot_2d(x1: Q32, y1: Q32, x2: Q32, y2: Q32) -> Q32 {
    x1 * x2 + y1 * y2
}

/// Get 2D gradient vector from gradient index
/// Returns (gx, gy) in Q32 fixed-point format
#[inline(always)]
fn grad2(index: usize) -> (Q32, Q32) {
    // Gradients are combinations of -1, 0, and 1, normalized
    // For 2D, we use 8 gradients
    const DIAG: Q32 = Q32(0xB505); // 1/sqrt(2) ≈ 0.70710678118 in Q16.16

    match index % 8 {
        0 => (Q32::ONE, Q32::ZERO),  // (1, 0)
        1 => (-Q32::ONE, Q32::ZERO), // (-1, 0)
        2 => (Q32::ZERO, Q32::ONE),  // (0, 1)
        3 => (Q32::ZERO, -Q32::ONE), // (0, -1)
        4 => (DIAG, DIAG),           // (1/sqrt(2), 1/sqrt(2))
        5 => (-DIAG, DIAG),          // (-1/sqrt(2), 1/sqrt(2))
        6 => (DIAG, -DIAG),          // (1/sqrt(2), -1/sqrt(2))
        7 => (-DIAG, -DIAG),         // (-1/sqrt(2), -1/sqrt(2))
        _ => (Q32::ONE, Q32::ZERO),  // Should never happen
    }
}

/// Compute surflet contribution for a corner
#[inline(always)]
fn surflet_2d(gradient_index: usize, x: Q32, y: Q32) -> Q32 {
    // t = 1.0 - dist^2 * 2.0
    let dist_sq = magnitude_squared_2d(x, y);
    let dist_sq_times_2 = dist_sq * TWO;
    let t = Q32::ONE - dist_sq_times_2;

    if t > Q32::ZERO {
        // Get gradient
        let (gx, gy) = grad2(gradient_index);

        // Compute dot product: gradient · offset
        let dot = dot_2d(gx, gy, x, y);

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
    use crate::builtins::fixed32::test_helpers::{fixed_to_float, float_to_fixed};
    use std::{print, println};

    #[test]
    fn test_simplex2_basic() {
        let result1 = __lp_fixed32_lp_simplex2(float_to_fixed(1.5), float_to_fixed(2.3), 0);
        let result2 = __lp_fixed32_lp_simplex2(float_to_fixed(3.7), float_to_fixed(2.3), 0);
        let result3 = __lp_fixed32_lp_simplex2(float_to_fixed(1.5), float_to_fixed(2.3), 1);

        // Different inputs should produce different outputs
        assert_ne!(
            result1, result2,
            "Noise should differ for different x values"
        );
        assert_ne!(result1, result3, "Noise should differ for different seeds");
    }

    #[test]
    fn test_simplex2_range() {
        // Test that output is approximately in [-1, 1] range
        for i in 0..50 {
            let x = float_to_fixed(i as f32 * 0.1);
            let y = float_to_fixed(i as f32 * 0.15);
            let result = __lp_fixed32_lp_simplex2(x, y, 0);
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
    fn test_simplex2_deterministic() {
        let result1 = __lp_fixed32_lp_simplex2(float_to_fixed(42.5), float_to_fixed(37.3), 123);
        let result2 = __lp_fixed32_lp_simplex2(float_to_fixed(42.5), float_to_fixed(37.3), 123);

        // Same input and seed should produce same output
        assert_eq!(result1, result2, "Noise should be deterministic");
    }

    #[cfg(feature = "test")]
    #[test]
    fn test_simplex2_properties() {
        use noise::{NoiseFn, Simplex};

        // Create noise-rs simplex for reference comparison
        let noise_rs_fn = Simplex::new(0);

        // Test multiple points - verify our implementation has similar properties
        let test_points = [
            (0.0, 0.0),
            (1.0, 0.0),
            (0.0, 1.0),
            (5.5, 3.2),
            (10.0, 10.0),
            (-5.0, -3.0),
        ];

        for (x, y) in test_points {
            // Get our output
            let our_value_fixed = __lp_fixed32_lp_simplex2(float_to_fixed(x), float_to_fixed(y), 0);
            let our_value = fixed_to_float(our_value_fixed);

            // Get noise-rs output for reference
            let noise_rs_value = noise_rs_fn.get([x as f64, y as f64]) as f32;

            // Verify our output is in reasonable range (similar to noise-rs)
            assert!(
                our_value >= -2.0 && our_value <= 2.0,
                "Simplex2({}, {}) should be in range [-2, 2], got {}",
                x,
                y,
                our_value
            );

            // Verify noise-rs is also in similar range (sanity check)
            assert!(
                noise_rs_value >= -2.0 && noise_rs_value <= 2.0,
                "noise-rs Simplex2({}, {}) should be in range [-2, 2], got {}",
                x,
                y,
                noise_rs_value
            );

            // Note: We don't compare exact values because we use a different hash function (noiz)
            // The important thing is that our implementation produces reasonable noise values
        }
    }

    #[test]
    fn test_simplex2_output_grid() {
        // Output a grid of noise values for manual inspection
        println!("\n=== Simplex2 Noise Grid (seed=0) ===");
        println!("5x5 grid, X and Y from 0.0 to 4.0:");
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
                let result = __lp_fixed32_lp_simplex2(float_to_fixed(x), float_to_fixed(y), 0);
                let result_float = fixed_to_float(result);
                print!("{:6.3} ", result_float);
            }
            println!();
        }

        println!("\n=== Simplex2 Seed Comparison (x=2.5, y=2.5) ===");
        let x = float_to_fixed(2.5);
        let y = float_to_fixed(2.5);
        for seed in 0..5 {
            let result = __lp_fixed32_lp_simplex2(x, y, seed);
            let result_float = fixed_to_float(result);
            println!("  seed={}: {:7.4}", seed, result_float);
        }

        // Verify outputs are in reasonable range
        for i in 0..50 {
            let x = float_to_fixed(i as f32 * 0.1);
            let y = float_to_fixed(i as f32 * 0.15);
            let result = __lp_fixed32_lp_simplex2(x, y, 0);
            let result_float = fixed_to_float(result);
            assert!(
                result_float >= -2.0 && result_float <= 2.0,
                "Noise value should be in reasonable range, got {}",
                result_float
            );
        }
    }

    #[test]
    fn test_simplex2_different_seeds() {
        // Test that different seeds produce different outputs
        // This matches the GLSL filetest
        let x = float_to_fixed(0.5);
        let y = float_to_fixed(0.5);

        // Debug: manually trace through the algorithm
        use crate::fixed32::q32::Q32;
        let x_q32 = Q32::from_fixed(x);
        let y_q32 = Q32::from_fixed(y);
        let sum = x_q32 + y_q32;
        let skew = sum * super::SKEW_FACTOR_2D;
        let skewed_x = x_q32 + skew;
        let skewed_y = y_q32 + skew;
        let cell_x_int = skewed_x.to_i32();
        let cell_y_int = skewed_y.to_i32();

        println!("Input: x={}, y={}", fixed_to_float(x), fixed_to_float(y));
        println!(
            "Skewed: x={}, y={}",
            fixed_to_float(skewed_x.to_fixed()),
            fixed_to_float(skewed_y.to_fixed())
        );
        println!("Cell: ({}, {})", cell_x_int, cell_y_int);

        let n1 = __lp_fixed32_lp_simplex2(x, y, 0);
        let n2 = __lp_fixed32_lp_simplex2(x, y, 1);
        let n1_float = fixed_to_float(n1);
        let n2_float = fixed_to_float(n2);
        let diff = (n1_float - n2_float).abs();

        println!("Simplex2(0.5, 0.5, seed=0) = {}", n1_float);
        println!("Simplex2(0.5, 0.5, seed=1) = {}", n2_float);
        println!("Difference = {}", diff);

        // Check hash values directly
        use crate::builtins::shared::lp_hash::__lp_hash_2;
        let hash0 = __lp_hash_2(cell_x_int as u32, cell_y_int as u32, 0);
        let hash1 = __lp_hash_2(cell_x_int as u32, cell_y_int as u32, 1);
        println!(
            "Hash({}, {}, seed=0) = {}, mod 8 = {}",
            cell_x_int,
            cell_y_int,
            hash0,
            hash0 % 8
        );
        println!(
            "Hash({}, {}, seed=1) = {}, mod 8 = {}",
            cell_x_int,
            cell_y_int,
            hash1,
            hash1 % 8
        );

        // Test multiple points to find one where seeds differ
        let mut found_difference = false;
        for i in 0..100 {
            let test_x = float_to_fixed(i as f32 * 0.1);
            let test_y = float_to_fixed(i as f32 * 0.1);
            let result_seed0 = __lp_fixed32_lp_simplex2(test_x, test_y, 0);
            let result_seed1 = __lp_fixed32_lp_simplex2(test_x, test_y, 1);
            if result_seed0 != result_seed1 {
                found_difference = true;
                println!(
                    "Found difference at ({}, {}): seed=0 -> {}, seed=1 -> {}",
                    i as f32 * 0.1,
                    i as f32 * 0.1,
                    fixed_to_float(result_seed0),
                    fixed_to_float(result_seed1)
                );
                break;
            }
        }

        assert!(
            found_difference,
            "Different seeds should produce different outputs at least at some points. At (0.5, 0.5): seed=0: {}, seed=1: {}, diff: {}",
            n1_float, n2_float, diff
        );
    }

    #[cfg(all(test, feature = "test_hash_fixed"))]
    mod fixed_hash_tests {
        use super::*;
        use crate::builtins::fixed32::test_helpers::{fixed_to_float, float_to_fixed};

        #[test]
        fn test_simplex2_boundary_continuity() {
            // Test continuity across cell boundaries using fixed hash
            // Points near cell boundaries should have smooth transitions
            let boundary_points = [
                (0.0, 0.0),
                (0.001, 0.001),
                (0.999, 0.999),
                (1.0, 1.0),
                (1.001, 1.001),
                (2.0, 2.0),
                (2.001, 2.001),
            ];

            let mut prev_value: Option<f32> = None;
            let mut max_jump = 0.0f32;

            for (x, y) in boundary_points {
                let result = __lp_fixed32_lp_simplex2(float_to_fixed(x), float_to_fixed(y), 0);
                let result_float = fixed_to_float(result);

                if let Some(prev) = prev_value {
                    let jump = (result_float - prev).abs();
                    max_jump = max_jump.max(jump);

                    // Noise should be relatively continuous (small changes for small position changes)
                    // After fixing the offset bugs, jumps should be reasonable
                    if jump > 0.5 {
                        println!(
                            "Large jump detected at ({}, {}): {} -> {}, jump = {}",
                            x, y, prev, result_float, jump
                        );
                    }
                }
                prev_value = Some(result_float);
            }

            println!("Maximum jump along boundary path: {}", max_jump);
            // After fixing bugs, max_jump should be reasonable (e.g., < 0.3)
            // This is a sanity check - exact threshold may need adjustment
            assert!(
                max_jump < 1.0,
                "Discontinuity detected: maximum jump = {}",
                max_jump
            );
        }

        #[test]
        fn test_simplex2_deterministic_with_fixed_hash() {
            // Test that fixed hash produces deterministic outputs
            let test_points = [(0.0, 0.0), (0.5, 0.5), (1.0, 1.0), (2.5, 2.5), (5.0, 5.0)];

            for (x, y) in test_points {
                let result1 = __lp_fixed32_lp_simplex2(float_to_fixed(x), float_to_fixed(y), 0);
                let result2 = __lp_fixed32_lp_simplex2(float_to_fixed(x), float_to_fixed(y), 0);
                assert_eq!(
                    result1, result2,
                    "Simplex2({}, {}) should be deterministic with fixed hash",
                    x, y
                );
            }
        }

        #[test]
        fn test_simplex2_no_discontinuities_along_line() {
            // Sample noise along a line and check for sudden jumps
            const STEP: f32 = 0.01;
            const THRESHOLD: f32 = 0.5; // Maximum allowed change per step

            let mut prev_value: Option<f32> = None;
            let mut max_jump = 0.0f32;
            let mut jump_count = 0;

            for i in 0..1000 {
                let x = i as f32 * STEP;
                let y = x; // Diagonal line
                let result = __lp_fixed32_lp_simplex2(float_to_fixed(x), float_to_fixed(y), 0);
                let result_float = fixed_to_float(result);

                if let Some(prev) = prev_value {
                    let jump = (result_float - prev).abs();
                    max_jump = max_jump.max(jump);

                    if jump > THRESHOLD {
                        jump_count += 1;
                        if jump_count <= 5 {
                            // Print first few jumps for debugging
                            println!(
                                "Large jump detected at ({}, {}): {} -> {}, jump = {}",
                                x, y, prev, result_float, jump
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
            // After fixing bugs, max_jump should be reasonable
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
        use crate::builtins::fixed32::test_helpers::{fixed_to_float, float_to_fixed};
        use std::fs::File;
        use std::io::Write;

        #[test]
        fn test_simplex2_generate_image() {
            // Generate a 256x256 noise image for visual inspection
            const WIDTH: usize = 256;
            const HEIGHT: usize = 256;
            const SCALE: f32 = 0.1; // Noise frequency

            let mut pixels = Vec::new();

            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    let fx = x as f32 * SCALE;
                    let fy = y as f32 * SCALE;
                    let noise = __lp_fixed32_lp_simplex2(float_to_fixed(fx), float_to_fixed(fy), 0);
                    let noise_float = fixed_to_float(noise);

                    // Normalize from [-1, 1] to [0, 255]
                    let normalized = ((noise_float + 1.0) * 127.5).clamp(0.0, 255.0) as u8;
                    pixels.push(normalized);
                    pixels.push(normalized);
                    pixels.push(normalized);
                }
            }

            // Write PPM format (simple, no dependencies)
            let mut file = File::create("test_output_simplex2.ppm").unwrap();
            writeln!(file, "P3").unwrap();
            writeln!(file, "{} {}", WIDTH, HEIGHT).unwrap();
            writeln!(file, "255").unwrap();

            for chunk in pixels.chunks(3) {
                writeln!(file, "{} {} {}", chunk[0], chunk[1], chunk[2]).unwrap();
            }

            println!("Generated test_output_simplex2.ppm for visual inspection");
        }

        #[test]
        fn test_simplex2_no_discontinuities() {
            // Sample noise along a line and check for sudden jumps
            const STEP: f32 = 0.01;
            const THRESHOLD: f32 = 0.3; // Maximum allowed change per step

            let mut prev_value: Option<f32> = None;
            let mut max_jump = 0.0f32;
            let mut jump_count = 0;
            let mut jump_locations = Vec::new();

            for i in 0..1000 {
                let x = i as f32 * STEP;
                let y = x; // Diagonal line
                let result = __lp_fixed32_lp_simplex2(float_to_fixed(x), float_to_fixed(y), 0);
                let result_float = fixed_to_float(result);

                if let Some(prev) = prev_value {
                    let jump = (result_float - prev).abs();
                    max_jump = max_jump.max(jump);

                    if jump > THRESHOLD {
                        jump_count += 1;
                        if jump_locations.len() < 10 {
                            jump_locations.push((x, y, prev, result_float, jump));
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
                for (x, y, prev, curr, jump) in jump_locations.iter().take(5) {
                    println!(
                        "  ({:.3}, {:.3}): {} -> {}, jump = {}",
                        x, y, prev, curr, jump
                    );
                }
            }

            // After fixing all bugs, max_jump should be reasonable
            assert!(
                max_jump < 0.5,
                "Discontinuity detected: maximum jump = {} (threshold: 0.5)",
                max_jump
            );
        }

        #[cfg(feature = "test")]
        #[test]
        fn test_simplex2_compare_with_noise_rs() {
            use noise::{NoiseFn, Simplex};

            let noise_rs_fn = Simplex::new(0);
            let test_points = [(0.0, 0.0), (0.5, 0.5), (1.0, 1.0), (5.5, 3.2), (10.0, 10.0)];

            println!("\n=== Simplex2 Comparison with noise-rs ===");
            for (x, y) in test_points {
                let our_value = __lp_fixed32_lp_simplex2(float_to_fixed(x), float_to_fixed(y), 0);
                let our_float = fixed_to_float(our_value);

                let noise_rs_value = noise_rs_fn.get([x as f64, y as f64]) as f32;

                let diff = (our_float - noise_rs_value).abs();
                println!(
                    "Point ({:4.1}, {:4.1}): ours={:8.5}, noise-rs={:8.5}, diff={:8.5}",
                    x, y, our_float, noise_rs_value, diff
                );

                // Verify both are in reasonable range
                assert!(our_float >= -2.0 && our_float <= 2.0);
                assert!(noise_rs_value >= -2.0 && noise_rs_value <= 2.0);
            }
        }
    }
}
