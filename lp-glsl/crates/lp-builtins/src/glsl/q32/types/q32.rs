/// Fixed-point arithmetic (16.16 format)
///
/// Core type and conversion utilities for fixed-point fixed.
use core::cmp::Ord;
use core::ops::{Add, Div, Mul, Neg, Sub};

/// Fixed-point constants
const SHIFT: i32 = 16;
const ONE: i32 = 1 << SHIFT;
const HALF: i32 = ONE / 2;

/// Fixed-point number (Q16.16 format)
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Q32(pub i32);

impl Q32 {
    // 2π ≈ 6.28318530718 in 16.16
    pub const E: Q32 = Q32(178145);
    pub const HALF: Q32 = Q32(HALF);
    pub const ONE: Q32 = Q32(ONE);
    // e ≈ 2.71828182846 in 16.16
    pub const PHI: Q32 = Q32(106039);
    // Mathematical constants
    pub const PI: Q32 = Q32(205887);
    pub const SHIFT: i32 = SHIFT;
    // π ≈ 3.14159265359 in 16.16
    pub const TAU: Q32 = Q32(411774);
    pub const ZERO: Q32 = Q32(0);

    // φ ≈ 1.61803398875 in 16.16 (golden ratio)

    /// Create a Fixed from a raw fixed-point value
    #[inline(always)]
    pub const fn from_fixed(f: i32) -> Self {
        Q32(f)
    }

    /// Create a Fixed from an f32
    #[inline(always)]
    pub fn from_f32(f: f32) -> Self {
        Q32((f * ONE as f32) as i32)
    }

    /// Create a Fixed from an i32
    #[inline(always)]
    pub const fn from_i32(i: i32) -> Self {
        Q32(i << Self::SHIFT)
    }

    /// Convert to f32
    #[inline(always)]
    pub fn to_f32(self) -> f32 {
        self.0 as f32 / ONE as f32
    }

    /// Get the raw fixed-point value
    #[inline(always)]
    pub const fn to_fixed(self) -> i32 {
        self.0
    }

    /// Clamp value between min and max
    #[inline(always)]
    pub fn clamp(self, min: Q32, max: Q32) -> Q32 {
        Q32(self.0.clamp(min.0, max.0))
    }

    /// Return the maximum of two values
    #[inline(always)]
    pub fn max(self, other: Q32) -> Q32 {
        Q32(self.0.max(other.0))
    }

    /// Return the minimum of two values
    #[inline(always)]
    pub fn min(self, other: Q32) -> Q32 {
        Q32(self.0.min(other.0))
    }

    /// Return the absolute value
    #[inline(always)]
    pub fn abs(self) -> Q32 {
        Q32(self.0.abs())
    }

    /// Check if value is zero
    #[inline(always)]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Get the fractional part (0..1)
    #[inline(always)]
    pub const fn frac(self) -> Q32 {
        Q32(self.0 & (ONE - 1))
    }

    /// Get the integer part (floor)
    #[inline(always)]
    pub const fn to_i32(self) -> i32 {
        self.0 >> Self::SHIFT
    }

    /// Multiply by an integer (more efficient than converting to Fixed first)
    #[inline(always)]
    pub const fn mul_int(self, i: i32) -> Q32 {
        Q32(self.0 * i)
    }
}

impl Add for Q32 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self {
        Q32(self.0 + rhs.0)
    }
}

impl Sub for Q32 {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self {
        Q32(self.0 - rhs.0)
    }
}

impl Mul for Q32 {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self {
        Q32(((self.0 as i64 * rhs.0 as i64) >> Self::SHIFT) as i32)
    }
}

impl Div for Q32 {
    type Output = Self;

    #[inline(always)]
    fn div(self, rhs: Self) -> Self {
        if rhs.0 != 0 {
            Q32(((self.0 as i64 * ONE as i64) / rhs.0 as i64) as i32)
        } else {
            Q32(0)
        }
    }
}

impl core::ops::Rem for Q32 {
    type Output = Self;

    #[inline(always)]
    fn rem(self, rhs: Self) -> Self {
        if rhs.0 != 0 {
            Q32(self.0 % rhs.0)
        } else {
            Q32(0)
        }
    }
}

impl Neg for Q32 {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self {
        Q32(-self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(Q32::ZERO.to_f32(), 0.0);
        assert_eq!(Q32::ONE.to_f32(), 1.0);
        assert_eq!(Q32::HALF.to_f32(), 0.5);
    }

    #[test]
    fn test_from_i32() {
        assert_eq!(Q32::from_i32(5).to_f32(), 5.0);
        assert_eq!(Q32::from_i32(-3).to_f32(), -3.0);
        assert_eq!(Q32::from_i32(0).to_f32(), 0.0);
    }

    #[test]
    fn test_from_f32() {
        let f = Q32::from_f32(1.5);
        assert!((f.to_f32() - 1.5).abs() < 0.001);

        let f2 = Q32::from_f32(-2.75);
        assert!((f2.to_f32() - (-2.75)).abs() < 0.001);
    }

    #[test]
    fn test_add() {
        let a = Q32::from_i32(2);
        let b = Q32::from_i32(3);
        assert_eq!((a + b).to_f32(), 5.0);
    }

    #[test]
    fn test_sub() {
        let a = Q32::from_i32(5);
        let b = Q32::from_i32(3);
        assert_eq!((a - b).to_f32(), 2.0);
    }

    #[test]
    fn test_mul() {
        let a = Q32::from_i32(2);
        let b = Q32::from_i32(3);
        assert_eq!((a * b).to_f32(), 6.0);

        let c = Q32::from_f32(1.5);
        let d = Q32::from_f32(2.0);
        assert!((c * d).to_f32() - 3.0 < 0.01);
    }

    #[test]
    fn test_div() {
        let a = Q32::from_i32(6);
        let b = Q32::from_i32(2);
        assert_eq!((a / b).to_f32(), 3.0);

        let c = Q32::from_i32(3);
        let d = Q32::from_i32(2);
        assert!((c / d).to_f32() - 1.5 < 0.01);
    }

    #[test]
    fn test_neg() {
        let a = Q32::from_i32(5);
        assert_eq!((-a).to_f32(), -5.0);

        let b = Q32::from_i32(-3);
        assert_eq!((-b).to_f32(), 3.0);
    }

    #[test]
    fn test_clamp() {
        let val = Q32::from_i32(5);
        let min = Q32::from_i32(0);
        let max = Q32::from_i32(10);
        assert_eq!(val.clamp(min, max).to_f32(), 5.0);

        let val2 = Q32::from_i32(-5);
        assert_eq!(val2.clamp(min, max).to_f32(), 0.0);

        let val3 = Q32::from_i32(15);
        assert_eq!(val3.clamp(min, max).to_f32(), 10.0);
    }

    #[test]
    fn test_min_max() {
        let a = Q32::from_i32(5);
        let b = Q32::from_i32(10);
        assert_eq!(a.min(b).to_f32(), 5.0);
        assert_eq!(a.max(b).to_f32(), 10.0);
    }
}
