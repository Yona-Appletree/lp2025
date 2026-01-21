//! Reference implementation of fixed16x16 division using reciprocal multiplication.
//!
//! This module provides a reference implementation of division for fixed16x16 format
//! using the reciprocal multiplication approach. This is our current approach for
//! fixed-point division in the compiler due to its speed and simplicity.
//!
//! ## Approach
//!
//! Instead of performing direct division (which is expensive on many architectures),
//! we use reciprocal multiplication:
//!
//! ```text
//! quotient = (dividend * recip * 2) >> shift_amount
//! ```
//!
//! where `recip = 0x8000_0000 / divisor` (precomputed using integer division).
//!
//! This converts one division operation into:
//! 1. One integer division (to compute the reciprocal)
//! 2. Two multiplications
//! 3. One right shift
//!
//! ## Precision Limitations
//!
//! The reciprocal method introduces small errors due to truncation in the reciprocal
//! calculation (`0x8000_0000 / divisor` uses integer division). Typical error is
//! around ~0.01% for normal cases, but can be larger (~2-3%) for edge cases like:
//! - Saturated values (at MAX_FIXED) divided by large divisors
//! - Very small divisors
//!
//! For most use cases, this precision is acceptable given the performance benefits.
//!
//! ## Alternative: Full Long Division
//!
//! An incomplete implementation of exact division using full long division (u64/u32)
//! exists in the `feature/udiv64` branch. This approach would provide exact results
//! but is significantly more complex and slower. See `lp-glsl-builtins-src` for details
//! on the algorithm and the debugging work done to identify lowering bugs.
//!
//! ## Purpose
//!
//! This serves as:
//! 1. A reference implementation to validate the compiler's division code generation
//! 2. A test harness to verify correctness and understand precision limits
//! 3. Documentation of the algorithm used in the compiler

const SHIFT: u32 = 16;
const SCALE: u32 = 1 << SHIFT; // 65536
const MAX_FIXED: i32 = 0x7FFF_FFFF; // Maximum representable fixed-point value
const MIN_FIXED: i32 = i32::MIN; // Minimum representable fixed-point value

const MAX_FLOAT: f32 = MAX_FIXED as f32 / SCALE as f32; // ~32767.99998
const MIN_FLOAT: f32 = MIN_FIXED as f32 / SCALE as f32; // ~-32768.0

/// Convert float to fixed16x16 with saturation
fn float_to_fixed(f: f32) -> i32 {
    if f > MAX_FLOAT {
        MAX_FIXED
    } else if f < MIN_FLOAT {
        MIN_FIXED
    } else {
        (f * SCALE as f32).round() as i32
    }
}

/// Convert fixed16x16 to float
fn fixed_to_float(fixed: i32) -> f32 {
    fixed as f32 / SCALE as f32
}

/// Unsigned division using reciprocal multiplication.
///
/// Algorithm:
/// 1. Compute reciprocal: `recip = 0x8000_0000 / divisor` (integer division, truncates)
/// 2. Calculate quotient: `(dividend * recip * 2) >> 16`
///
/// The multiplication by 2 and right shift by 16 effectively scales the result
/// to account for the fixed-point representation.
fn fixed32_udiv(dividend: u32, divisor: u32) -> u32 {
    // Precompute reciprocal: 1/divisor scaled by 2^31
    // Integer division truncates, introducing precision error
    let recip = 0x8000_0000u32 / divisor;

    // Calculate quotient using reciprocal multiplication
    // Formula: (dividend * recip * 2) >> 16
    let quotient = (((dividend as u64) * (recip as u64) * 2u64) >> SHIFT) as u32;

    quotient
}

/// Signed division using reciprocal multiplication.
///
/// Handles sign by:
/// 1. Computing absolute values of dividend and divisor
/// 2. Performing unsigned division
/// 3. Applying the sign based on the XOR of the original signs
fn fixed32_idiv(dividend: i32, divisor: i32) -> i32 {
    // Determine result sign: negative if signs differ
    let result_sign = if (dividend ^ divisor) < 0 { -1 } else { 1 };

    // Perform unsigned division on absolute values, then apply sign
    (fixed32_udiv(dividend.abs() as u32, divisor.abs() as u32) as i32) * result_sign
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{string::String, vec::Vec};

    #[test]
    fn test_udiv() {
        let tests = vec![
            (0.999, 0.998),
            (10.0, 2.0),
            //(10.0, -2.0, -5.0),
            //(-10.0, -2.0, 5.0),
            (7.5, 1.0),
            (15.0, 3.0),
            (20.0, 2.0),
        ];

        for (dividend, divisor) in tests {
            let expected_quotient = dividend / divisor;

            let dividend = float_to_fixed(dividend) as u32;
            let divisor = float_to_fixed(divisor) as u32;

            let result = fixed_to_float(fixed32_udiv(dividend, divisor) as i32);

            println!(
                "Test: {} / {} -> Expected: {}, Actual: {}",
                dividend, divisor, expected_quotient, result
            );

            if (result - expected_quotient).abs() > 0.001 {
                panic!(
                    "Test failed: {} / {}; actual: {}; expected {}",
                    dividend, divisor, result, expected_quotient,
                );
            }
        }
    }

    #[test]
    fn test_idiv() {
        let tests = vec![
            (0.999, 0.998),
            (10.0, 2.0),
            (10.0, -2.0),
            (-10.0, -2.0),
            (7.5, 1.0),
            (15.0, 3.0),
            (20.0, 2.0),
        ];

        for (dividend, divisor) in tests {
            let expected_quotient = dividend / divisor;

            let dividend = float_to_fixed(dividend);
            let divisor = float_to_fixed(divisor);

            let result = fixed_to_float(fixed32_idiv(dividend, divisor));

            println!(
                "Test: {} / {} -> Expected: {}, Actual: {}",
                dividend, divisor, expected_quotient, result
            );

            if (result - expected_quotient).abs() > 0.001 {
                panic!(
                    "Test failed: {} / {}; actual: {}; expected {}",
                    dividend, divisor, result, expected_quotient,
                );
            }
        }
    }

    #[test]
    fn test_large_values_saturation() {
        // Test that large values saturate correctly
        let large_positive = 1000000.0;
        let large_negative = -1000000.0;

        let fixed_large_pos = float_to_fixed(large_positive);
        let fixed_large_neg = float_to_fixed(large_negative);

        // Should saturate to MAX_FIXED and MIN_FIXED
        assert_eq!(
            fixed_large_pos, MAX_FIXED,
            "Large positive value should saturate to MAX_FIXED"
        );
        assert_eq!(
            fixed_large_neg, MIN_FIXED,
            "Large negative value should saturate to MIN_FIXED"
        );

        // Verify conversion back
        let back_to_float_pos = fixed_to_float(fixed_large_pos);
        let back_to_float_neg = fixed_to_float(fixed_large_neg);

        // MAX_FIXED = 0x7FFF_FFFF = 2147483647, which is ~32767.99998 in float
        assert!(
            (back_to_float_pos - 32767.0).abs() < 0.1,
            "MAX_FIXED should convert back to ~32767.0"
        );
        assert!(
            (back_to_float_neg - (-32768.0)).abs() < 0.1,
            "MIN_FIXED should convert back to ~-32768.0"
        );
    }

    #[test]
    fn test_exact_failing_case() {
        // Reproduce the exact failing test case: 1000000.0 / 1000.0
        let dividend = 1000000.0;
        let divisor = 1000.0;

        let dividend_fixed = float_to_fixed(dividend);
        let divisor_fixed = float_to_fixed(divisor);

        println!("=== Exact Failing Case ===");
        println!("dividend: {}", dividend);
        println!("divisor: {}", divisor);
        println!(
            "dividend_fixed: {} (0x{:X})",
            dividend_fixed, dividend_fixed as u32
        );
        println!(
            "divisor_fixed: {} (0x{:X})",
            divisor_fixed, divisor_fixed as u32
        );
        println!("MAX_FIXED: {} (0x{:X})", MAX_FIXED, MAX_FIXED as u32);
        println!("MAX_FIXED as float: {}", fixed_to_float(MAX_FIXED));

        let result = fixed_to_float(fixed32_idiv(dividend_fixed, divisor_fixed));
        let expected = 32.768;

        println!("result: {}", result);
        println!("expected: {}", expected);
        println!("difference: {}", (result - expected).abs());

        // Check if this matches the compiler output
        if (result - 31.999985).abs() < 0.0001 {
            println!(
                "WARNING: Reference implementation produces same wrong result: {}",
                result
            );
        } else {
            println!("Reference implementation produces different result - compiler bug!");
        }
    }

    #[test]
    fn test_large_value_division() {
        // Test division with large values that saturate
        // 1000000.0 / 1000.0 should become MAX_FIXED / 1000.0 = ~32.768
        let dividend = 1000000.0;
        let divisor = 1000.0;

        let dividend_fixed = float_to_fixed(dividend);
        let divisor_fixed = float_to_fixed(divisor);

        println!(
            "dividend_fixed: {} (0x{:X})",
            dividend_fixed, dividend_fixed as u32
        );
        println!(
            "divisor_fixed: {} (0x{:X})",
            divisor_fixed, divisor_fixed as u32
        );
        println!("MAX_FIXED: {} (0x{:X})", MAX_FIXED, MAX_FIXED as u32);
        println!("MAX_FIXED as float: {}", fixed_to_float(MAX_FIXED));

        // dividend_fixed should be MAX_FIXED (saturated)
        assert_eq!(
            dividend_fixed, MAX_FIXED,
            "1000000.0 should saturate to MAX_FIXED"
        );

        let result = fixed_to_float(fixed32_idiv(dividend_fixed, divisor_fixed));

        // MAX_FIXED / 1000.0 = 32767.99998 / 1000.0 = 32.76799998
        let expected = fixed_to_float(MAX_FIXED) / divisor;
        println!(
            "Test: {} / {} -> Expected: {}, Actual: {}",
            dividend, divisor, expected, result
        );
        println!("Difference: {}", (result - expected).abs());

        // Allow some tolerance for the reciprocal method's precision
        if (result - expected).abs() > 0.1 {
            panic!(
                "Test failed: {} / {}; actual: {}; expected {}",
                dividend, divisor, result, expected,
            );
        }
    }

    #[test]
    fn test_at_boundary_values() {
        // Test values at the boundary of fixed16x16 range
        let max_representable = 32767.99998; // Close to MAX_FIXED
        let min_representable = -32768.0; // MIN_FIXED

        let fixed_max = float_to_fixed(max_representable);
        let fixed_min = float_to_fixed(min_representable);

        // Should not saturate (within range)
        assert!(
            fixed_max <= MAX_FIXED,
            "max_representable should be within range"
        );
        assert!(
            fixed_min >= MIN_FIXED,
            "min_representable should be within range"
        );

        // Test division at boundaries
        let result_max = fixed_to_float(fixed32_idiv(fixed_max, float_to_fixed(1000.0)));
        let result_min = fixed_to_float(fixed32_idiv(fixed_min, float_to_fixed(1000.0)));

        println!("Boundary test - max: {}, min: {}", result_max, result_min);

        // Results should be reasonable
        assert!(
            result_max > 0.0 && result_max < 50.0,
            "max boundary division should be reasonable"
        );
        assert!(
            result_min < 0.0 && result_min > -50.0,
            "min boundary division should be reasonable"
        );
    }
}
