//! 64-bit dividend by 32-bit divisor division algorithm
//!
//! This implementation is adapted from Rust's compiler-builtins crate:
//! https://github.com/rust-lang/compiler-builtins/blob/main/compiler-builtins/src/int/specialized_div_rem/delegate.rs
//!
//! Specifically, this uses the `impl_delegate!` algorithm for u64_div_rem on 32-bit targets.
//! The algorithm handles the case where:
//! - Dividend is 64-bit (split into dividend_hi and dividend_lo, both 32-bit)
//! - Divisor is 32-bit
//! - Result is 32-bit quotient
//!
//! This is the same algorithm that Rust uses when targeting riscv32imac and performing
//! 64-bit unsigned division operations.

/// Normalization shift function for 32-bit values.
///
/// Finds the shift left that the divisor `div` would need to be normalized for a binary
/// long division step with the dividend `duo`.
///
/// Adapted from compiler-builtins/src/int/specialized_div_rem/norm_shift.rs
#[allow(dead_code)]
fn u32_normalization_shift(duo: u32, div: u32, full_normalization: bool) -> usize {
    // Use leading_zeros since RISC-V has CLZ instruction (or we can use software fallback)
    let mut shl = (div.leading_zeros() - duo.leading_zeros()) as usize;
    if full_normalization {
        if duo < (div << shl) {
            // When the msb of `duo` and `div` are aligned, the resulting `div` may be
            // larger than `duo`, so we decrease the shift by 1.
            shl -= 1;
        }
    }
    shl
}

/// 32-bit by 32-bit division helper.
///
/// This delegates to hardware division when available, or uses software division.
#[allow(dead_code)]
fn u32_by_u32_div_rem(duo: u32, div: u32) -> (u32, u32) {
    // Use checked_div/checked_rem to avoid panic dependencies
    if let Some(quo) = duo.checked_div(div) {
        if let Some(rem) = duo.checked_rem(div) {
            return (quo, rem);
        }
    }
    // Division by zero - this should not happen in our use case
    // but we need to handle it to match the compiler-builtins interface
    // In no_std, we can't use unreachable, so we'll panic
    panic!("division by zero")
}

/// Divide a 64-bit unsigned dividend by a 32-bit unsigned divisor
/// Returns the 32-bit quotient
///
/// # Panics
/// Panics if dividend_hi >= divisor (quotient would overflow 32 bits)
///
/// This implementation is adapted from Rust's compiler-builtins delegate algorithm
/// for u64_div_rem on 32-bit targets. See:
/// https://github.com/rust-lang/compiler-builtins/blob/main/compiler-builtins/src/int/specialized_div_rem/delegate.rs
#[allow(dead_code)]
pub fn divide64(dividend_hi: u32, dividend_lo: u32, divisor: u32) -> u32 {
    // Check that dividend_hi < divisor to avoid quotient overflow
    assert!(dividend_hi < divisor, "Quotient would overflow 32 bits");

    // Build 64-bit dividend
    let duo = ((dividend_hi as u64) << 32) | (dividend_lo as u64);
    let div = divisor as u64;

    // Split into high and low 32-bit parts
    let duo_lo = duo as u32;
    let duo_hi = (duo >> 32) as u32;
    let div_lo = div as u32;
    let div_hi = (div >> 32) as u32;

    // Match the cases from compiler-builtins delegate.rs
    // The algorithm handles different cases based on whether div_lo, div_hi, and duo_hi are zero
    match (div_lo == 0, div_hi == 0, duo_hi == 0) {
        (true, true, _) => {
            // Division by zero - should not happen due to our assertion
            panic!("division by zero")
        }
        (_, false, true) => {
            // `duo` < `div` - quotient is 0
            return 0;
        }
        (false, true, true) => {
            // Both dividend and divisor fit in 32 bits - delegate to 32-bit division
            let tmp = u32_by_u32_div_rem(duo_lo, div_lo);
            return tmp.0;
        }
        (false, true, false) => {
            // 64-bit dividend, 32-bit divisor
            // This is our main case
            if duo_hi < div_lo {
                // `quo_hi` will always be 0. This performs a binary long division algorithm
                // to zero `duo_hi` followed by a half division.

                // We can calculate the normalization shift using only 32-bit functions.
                // If we calculated the normalization shift using
                // `u32_normalization_shift(duo_hi, div_lo, false)`, it would break the
                // assumption the function has that the first argument is more than the
                // second argument. If the arguments are switched, the assumption holds true
                // since `duo_hi < div_lo`.
                let norm_shift = u32_normalization_shift(div_lo, duo_hi, false);
                let n = 32; // number of bits in u32
                let shl = if norm_shift == 0 {
                    // Consider what happens if the msbs of `duo_hi` and `div_lo` align with
                    // no shifting. The normalization shift will always return
                    // `norm_shift == 0` regardless of whether it is fully normalized,
                    // because `duo_hi < div_lo`. In that edge case, `n - norm_shift` would
                    // result in shift overflow down the line. For the edge case, because
                    // both `duo_hi < div_lo` and we are comparing all the significant bits
                    // of `duo_hi` and `div`, we can make `shl = n - 1`.
                    n - 1
                } else {
                    // We also cannot just use `shl = n - norm_shift - 1` in the general
                    // case, because when we are not in the edge case comparing all the
                    // significant bits, then the full `duo < div` may not be true and thus
                    // breaks the division algorithm.
                    n - norm_shift
                };

                // The 3 variable restoring division algorithm is ideal for this task,
                // since `pow` and `quo` can be u32 and the delegation check is simple.
                let mut div_shifted: u64 = div << shl;
                let mut pow_lo: u32 = 1 << shl;
                let mut quo_lo: u32 = 0;
                let mut duo = duo;
                loop {
                    let sub = duo.wrapping_sub(div_shifted);
                    if 0 <= (sub as i64) {
                        duo = sub;
                        quo_lo |= pow_lo;
                        let duo_hi = (duo >> 32) as u32;
                        if duo_hi == 0 {
                            // Delegate to get the rest of the quotient. Note that the
                            // `div_lo` here is the original unshifted `div`.
                            let tmp = u32_by_u32_div_rem(duo as u32, div_lo);
                            return quo_lo | tmp.0;
                        }
                    }
                    div_shifted >>= 1;
                    pow_lo >>= 1;
                }
            } else if duo_hi == div_lo {
                // `quo_hi == 1`. This branch is cheap and helps with edge cases.
                let tmp = u32_by_u32_div_rem(duo as u32, div as u32);
                return ((1u64 << 32) | (tmp.0 as u64)) as u32;
            } else {
                // `div_lo < duo_hi`
                // `rem_hi == 0`
                let n_h = 16; // half the number of bits in u32
                if (div_lo >> n_h) == 0 {
                    // Short division of u64 by a u16, using u32 by u32 division
                    let div_0 = div_lo as u16 as u32;
                    let (quo_hi, rem_3) = u32_by_u32_div_rem(duo_hi, div_0);

                    let duo_mid = ((duo >> n_h) as u16 as u32) | (rem_3 << n_h);
                    let (quo_1, rem_2) = u32_by_u32_div_rem(duo_mid, div_0);

                    let duo_lo = (duo as u16 as u32) | (rem_2 << n_h);
                    let (quo_0, _rem_1) = u32_by_u32_div_rem(duo_lo, div_0);

                    return ((quo_0 as u64) | ((quo_1 as u64) << n_h) | ((quo_hi as u64) << 32))
                        as u32;
                }

                // This is basically a short division composed of a half division for the hi
                // part, specialized 3 variable binary long division in the middle, and
                // another half division for the lo part.
                let duo_lo = duo as u32;
                let tmp = u32_by_u32_div_rem(duo_hi, div_lo);
                let quo_hi = tmp.0;
                let mut duo = (duo_lo as u64) | ((tmp.1 as u64) << 32);
                // This check is required to avoid breaking the long division below.
                if duo < div {
                    return ((quo_hi as u64) << 32) as u32;
                }

                // The half division handled all shift alignments down to `n`, so this
                // division can continue with a shift of `n - 1`.
                let n = 32;
                let mut div_shifted: u64 = div << (n - 1);
                let mut pow_lo: u32 = 1 << (n - 1);
                let mut quo_lo: u32 = 0;
                loop {
                    let sub = duo.wrapping_sub(div_shifted);
                    if 0 <= (sub as i64) {
                        duo = sub;
                        quo_lo |= pow_lo;
                        let duo_hi = (duo >> 32) as u32;
                        if duo_hi == 0 {
                            // Delegate to get the rest of the quotient. Note that the
                            // `div_lo` here is the original unshifted `div`.
                            let tmp = u32_by_u32_div_rem(duo as u32, div_lo);
                            return (tmp.0 as u64 | quo_lo as u64 | ((quo_hi as u64) << 32)) as u32;
                        }
                    }
                    div_shifted >>= 1;
                    pow_lo >>= 1;
                }
            }
        }
        (_, false, false) => {
            // Full 64-bit by 64-bit binary long division. `quo_hi` will always be 0.
            if duo < div {
                return 0;
            }
            let div_original = div;
            let shl = u32_normalization_shift(duo_hi, div_hi, false);
            let mut duo = duo;
            let mut div_shifted: u64 = div << shl;
            let mut pow_lo: u32 = 1u32.wrapping_shl(shl as u32);
            let mut quo_lo: u32 = 0;
            loop {
                let sub = duo.wrapping_sub(div_shifted);
                if 0 <= (sub as i64) {
                    duo = sub;
                    quo_lo |= pow_lo;
                    if duo < div_original {
                        return quo_lo;
                    }
                }
                div_shifted >>= 1;
                pow_lo >>= 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{string::String, vec::Vec};

    #[test]
    fn test_simple_division() {
        // Test: (0x000a_0000 << 16) / 0x0002_8000
        // = 0x000a_0000_0000 / 0x0002_8000
        // = 0x0004_0000
        // As 64-bit dividend: hi=0x0000_000a, lo=0x0000_0000
        let dividend_hi = 0x0000_000a;
        let dividend_lo = 0x0000_0000;
        let divisor = 0x0002_8000;
        let result = divide64(dividend_hi, dividend_lo, divisor);
        assert_eq!(result, 0x0004_0000, "Expected 4.0 in fixed-point");
    }

    #[test]
    fn test_division_with_high_bits() {
        // Test: (1 << 32) / 2 = 2^31
        let dividend_hi = 0x0000_0001;
        let dividend_lo = 0x0000_0000;
        let divisor = 0x0000_0002;
        let result = divide64(dividend_hi, dividend_lo, divisor);
        assert_eq!(result, 0x8000_0000);
    }

    #[test]
    #[should_panic(expected = "Quotient would overflow")]
    fn test_overflow_case() {
        // This should panic because dividend_hi >= divisor
        let dividend_hi = 0x0000_0002;
        let dividend_lo = 0x0000_0000;
        let divisor = 0x0000_0002;
        divide64(dividend_hi, dividend_lo, divisor);
    }

    /// Helper function to compute 64-bit dividend from hi and lo
    fn make_dividend(hi: u32, lo: u32) -> u64 {
        ((hi as u64) << 32) | (lo as u64)
    }

    /// Helper function to split 64-bit value into hi and lo
    fn split_dividend(dividend: u64) -> (u32, u32) {
        ((dividend >> 32) as u32, (dividend & 0xFFFF_FFFF) as u32)
    }

    #[test]
    fn test_comprehensive_range() {
        // Comprehensive test comparing divide64 to built-in division
        // This uses the same algorithm as Rust's compiler-builtins
        let mut test_count = 0;
        let mut failures: Vec<String> = Vec::new();

        // Test 1: Small values (1-1000)
        for divisor in 1..=1000u32 {
            for dividend_val in 1..(divisor as u64 * 1000) {
                if dividend_val >= (divisor as u64) << 32 {
                    break; // Skip if dividend_hi would be >= divisor
                }
                let (dividend_hi, dividend_lo) = split_dividend(dividend_val);
                if dividend_hi >= divisor {
                    continue; // Skip invalid cases
                }

                let expected = (dividend_val / divisor as u64) as u32;
                let result = divide64(dividend_hi, dividend_lo, divisor);

                test_count += 1;
                if result != expected {
                    failures.push(format!(
                        "dividend={} (hi={}, lo={}), divisor={}, expected={}, got={}",
                        dividend_val, dividend_hi, dividend_lo, divisor, expected, result
                    ));
                    if failures.len() > 10 {
                        break; // Stop after 10 failures
                    }
                }
            }
            if !failures.is_empty() {
                break;
            }
        }

        // Test 2: Powers of 2
        for shift in 0..=31 {
            let divisor = 1u32 << shift;
            if divisor == 0 {
                continue;
            }
            for dividend_shift in 0..=63 {
                let dividend_val = 1u64 << dividend_shift;
                if dividend_val >= (divisor as u64) << 32 {
                    continue;
                }
                let (dividend_hi, dividend_lo) = split_dividend(dividend_val);
                if dividend_hi >= divisor {
                    continue;
                }

                let expected = (dividend_val / divisor as u64) as u32;
                let result = divide64(dividend_hi, dividend_lo, divisor);

                test_count += 1;
                if result != expected {
                    failures.push(format!(
                        "power-of-2: dividend=2^{} (hi={}, lo={}), divisor=2^{}, expected={}, got={}",
                        dividend_shift, dividend_hi, dividend_lo, shift, expected, result
                    ));
                }
            }
        }

        // Test 3: Fixed-point common values (scaled by 2^16)
        let fixed_point_values = [
            0x0001_0000, // 1.0
            0x0002_0000, // 2.0
            0x000a_0000, // 10.0
            0x0064_0000, // 100.0
            0x03e8_0000, // 1000.0
            0x7fff_0000, // ~32767.0 (max fixed-point)
        ];

        for &dividend_fixed in &fixed_point_values {
            // Shift left by 16 to simulate (a << 16) / b
            let dividend_val = (dividend_fixed as u64) << 16;
            if dividend_val >= u64::MAX {
                continue;
            }
            let (dividend_hi, dividend_lo) = split_dividend(dividend_val);

            for &divisor_fixed in &fixed_point_values {
                if divisor_fixed == 0 {
                    continue;
                }
                if dividend_hi >= divisor_fixed {
                    continue;
                }

                let expected = (dividend_val / divisor_fixed as u64) as u32;
                let result = divide64(dividend_hi, dividend_lo, divisor_fixed);

                test_count += 1;
                if result != expected {
                    failures.push(format!(
                        "fixed-point: dividend={:x} (hi={}, lo={}), divisor={:x}, expected={}, got={}",
                        dividend_fixed, dividend_hi, dividend_lo, divisor_fixed, expected, result
                    ));
                }
            }
        }

        // Test 4: Edge cases - dividend_hi close to divisor
        for divisor in [100, 1000, 10000, 0x1000, 0x10000, 0x100000, 0x1000000] {
            if divisor == 0 {
                continue;
            }
            // Test dividend_hi = divisor - 1, divisor - 2, etc.
            for offset in 1..=10 {
                if offset >= divisor {
                    break;
                }
                let dividend_hi = divisor - offset;
                // Test with various dividend_lo values
                for dividend_lo_shift in [0, 1, 8, 16, 24, 31] {
                    let dividend_lo = if dividend_lo_shift < 32 {
                        1u32 << dividend_lo_shift
                    } else {
                        0xFFFF_FFFF
                    };
                    let dividend_val = make_dividend(dividend_hi, dividend_lo);

                    let expected = (dividend_val / divisor as u64) as u32;
                    let result = divide64(dividend_hi, dividend_lo, divisor);

                    test_count += 1;
                    if result != expected {
                        failures.push(format!(
                            "edge case: dividend_hi={} (close to divisor {}), dividend_lo={:x}, expected={}, got={}",
                            dividend_hi, divisor, dividend_lo, expected, result
                        ));
                        if failures.len() > 10 {
                            break;
                        }
                    }
                }
                if failures.len() > 10 {
                    break;
                }
            }
            if failures.len() > 10 {
                break;
            }
        }

        // Test 5: Large values (testing upper range)
        for divisor_exp in 16..=31 {
            let divisor = 1u32 << divisor_exp;
            if divisor == 0 {
                continue;
            }
            // Test with dividend_hi = divisor / 2, divisor / 4, etc.
            for dividend_hi_ratio in [2, 4, 8, 16] {
                let dividend_hi = divisor / dividend_hi_ratio;
                if dividend_hi == 0 {
                    continue;
                }
                // Test with various dividend_lo values
                for dividend_lo in [0, 1, 0xFFFF, 0xFFFF_FFFF] {
                    let dividend_val = make_dividend(dividend_hi, dividend_lo);

                    let expected = (dividend_val / divisor as u64) as u32;
                    let result = divide64(dividend_hi, dividend_lo, divisor);

                    test_count += 1;
                    if result != expected {
                        failures.push(format!(
                            "large values: dividend_hi={}, dividend_lo={:x}, divisor={}, expected={}, got={}",
                            dividend_hi, dividend_lo, divisor, expected, result
                        ));
                        if failures.len() > 10 {
                            break;
                        }
                    }
                }
                if failures.len() > 10 {
                    break;
                }
            }
            if failures.len() > 10 {
                break;
            }
        }

        // Test 6: Random-like patterns (using deterministic "random" values)
        let mut seed: u64 = 12345;
        for _ in 0..1000 {
            // Simple LCG for deterministic "random" values
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let divisor = ((seed >> 16) as u32) & 0x7FFF_FFFF;
            if divisor < 2 {
                continue;
            }

            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let dividend_hi = ((seed >> 16) as u32) % divisor;
            if dividend_hi >= divisor {
                continue;
            }

            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let dividend_lo = (seed >> 32) as u32;

            let dividend_val = make_dividend(dividend_hi, dividend_lo);
            let expected = (dividend_val / divisor as u64) as u32;
            let result = divide64(dividend_hi, dividend_lo, divisor);

            test_count += 1;
            if result != expected {
                failures.push(format!(
                    "random: dividend_hi={}, dividend_lo={:x}, divisor={}, expected={}, got={}",
                    dividend_hi, dividend_lo, divisor, expected, result
                ));
                if failures.len() > 10 {
                    break;
                }
            }
        }

        // Report results
        if !failures.is_empty() {
            eprintln!("Failed {}/{} tests:", failures.len(), test_count);
            for failure in &failures {
                eprintln!("  {}", failure);
            }
            panic!("divide64 failed {} test cases", failures.len());
        }

        eprintln!("Passed all {} comprehensive tests", test_count);
    }
}
