use core::ops::{Add, Div, Mul, Neg, Sub};

use super::q32::Q32;
use super::vec2_q32::Vec2Q32;
use crate::builtins::q32::__lp_q32_sqrt;

/// 3D vector for Q32 fixed-point arithmetic
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Vec3Q32 {
    pub x: Q32,
    pub y: Q32,
    pub z: Q32,
}

impl Vec3Q32 {
    #[inline(always)]
    pub const fn new(x: Q32, y: Q32, z: Q32) -> Self {
        Vec3Q32 { x, y, z }
    }

    #[inline(always)]
    pub fn from_f32(x: f32, y: f32, z: f32) -> Self {
        Vec3Q32 {
            x: Q32::from_f32(x),
            y: Q32::from_f32(y),
            z: Q32::from_f32(z),
        }
    }

    #[inline(always)]
    pub fn from_i32(x: i32, y: i32, z: i32) -> Self {
        Vec3Q32 {
            x: Q32::from_i32(x),
            y: Q32::from_i32(y),
            z: Q32::from_i32(z),
        }
    }

    #[inline(always)]
    pub const fn zero() -> Self {
        Vec3Q32::new(Q32::ZERO, Q32::ZERO, Q32::ZERO)
    }

    #[inline(always)]
    pub const fn one() -> Self {
        Vec3Q32::new(Q32::ONE, Q32::ONE, Q32::ONE)
    }

    /// Dot product
    #[inline(always)]
    pub fn dot(self, rhs: Self) -> Q32 {
        (self.x * rhs.x) + (self.y * rhs.y) + (self.z * rhs.z)
    }

    /// Cross product
    #[inline(always)]
    pub fn cross(self, rhs: Self) -> Self {
        Vec3Q32::new(
            (self.y * rhs.z) - (self.z * rhs.y),
            (self.z * rhs.x) - (self.x * rhs.z),
            (self.x * rhs.y) - (self.y * rhs.x),
        )
    }

    /// Length squared (avoids sqrt)
    #[inline(always)]
    pub fn length_squared(self) -> Q32 {
        self.dot(self)
    }

    /// Length
    #[inline(always)]
    pub fn length(self) -> Q32 {
        let len_sq = self.length_squared();
        Q32::from_fixed(__lp_q32_sqrt(len_sq.to_fixed()))
    }

    /// Distance between two vectors
    #[inline(always)]
    pub fn distance(self, other: Self) -> Q32 {
        (self - other).length()
    }

    /// Normalize (returns zero vector if length is zero)
    #[inline(always)]
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len.to_fixed() == 0 {
            return Vec3Q32::zero();
        }
        self / len
    }

    /// Reflect vector around normal
    #[inline(always)]
    pub fn reflect(self, normal: Self) -> Self {
        // reflect = v - 2 * dot(v, n) * n
        let dot_2 = self.dot(normal) * Q32::from_fixed(2 << 16);
        self - (normal * dot_2)
    }

    // Swizzle accessors (GLSL-style) - scalar
    #[inline(always)]
    pub fn x(self) -> Q32 {
        self.x
    }

    #[inline(always)]
    pub fn y(self) -> Q32 {
        self.y
    }

    #[inline(always)]
    pub fn z(self) -> Q32 {
        self.z
    }

    #[inline(always)]
    pub fn r(self) -> Q32 {
        self.x
    }

    #[inline(always)]
    pub fn g(self) -> Q32 {
        self.y
    }

    #[inline(always)]
    pub fn b(self) -> Q32 {
        self.z
    }

    // 2-component swizzles (most common)
    #[inline(always)]
    pub fn xy(self) -> Vec2Q32 {
        Vec2Q32::new(self.x, self.y)
    }

    #[inline(always)]
    pub fn xz(self) -> Vec2Q32 {
        Vec2Q32::new(self.x, self.z)
    }

    #[inline(always)]
    pub fn yz(self) -> Vec2Q32 {
        Vec2Q32::new(self.y, self.z)
    }

    #[inline(always)]
    pub fn yx(self) -> Vec2Q32 {
        Vec2Q32::new(self.y, self.x)
    }

    #[inline(always)]
    pub fn zx(self) -> Vec2Q32 {
        Vec2Q32::new(self.z, self.x)
    }

    #[inline(always)]
    pub fn zy(self) -> Vec2Q32 {
        Vec2Q32::new(self.z, self.y)
    }

    // 3-component swizzles (permutations)
    #[inline(always)]
    pub fn xyz(self) -> Vec3Q32 {
        self
    }

    // identity
    #[inline(always)]
    pub fn xzy(self) -> Vec3Q32 {
        Vec3Q32::new(self.x, self.z, self.y)
    }

    #[inline(always)]
    pub fn yxz(self) -> Vec3Q32 {
        Vec3Q32::new(self.y, self.x, self.z)
    }

    #[inline(always)]
    pub fn yzx(self) -> Vec3Q32 {
        Vec3Q32::new(self.y, self.z, self.x)
    }

    #[inline(always)]
    pub fn zxy(self) -> Vec3Q32 {
        Vec3Q32::new(self.z, self.x, self.y)
    }

    #[inline(always)]
    pub fn zyx(self) -> Vec3Q32 {
        Vec3Q32::new(self.z, self.y, self.x)
    }

    // RGBA variants
    #[inline(always)]
    pub fn rg(self) -> Vec2Q32 {
        self.xy()
    }

    #[inline(always)]
    pub fn rb(self) -> Vec2Q32 {
        self.xz()
    }

    #[inline(always)]
    pub fn gb(self) -> Vec2Q32 {
        self.yz()
    }

    #[inline(always)]
    pub fn rgb(self) -> Vec3Q32 {
        self
    }

    /// Component-wise multiply
    #[inline(always)]
    pub fn mul_comp(self, rhs: Self) -> Self {
        Vec3Q32::new(self.x * rhs.x, self.y * rhs.y, self.z * rhs.z)
    }

    /// Component-wise divide
    #[inline(always)]
    pub fn div_comp(self, rhs: Self) -> Self {
        Vec3Q32::new(self.x / rhs.x, self.y / rhs.y, self.z / rhs.z)
    }

    /// Clamp components between min and max
    #[inline(always)]
    pub fn clamp(self, min: Q32, max: Q32) -> Self {
        Vec3Q32::new(
            self.x.clamp(min, max),
            self.y.clamp(min, max),
            self.z.clamp(min, max),
        )
    }
}

// Vector + Vector
impl Add for Vec3Q32 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self {
        Vec3Q32::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

// Vector - Vector
impl Sub for Vec3Q32 {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self {
        Vec3Q32::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

// Vector * Scalar
impl Mul<Q32> for Vec3Q32 {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Q32) -> Self {
        Vec3Q32::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

// Vector / Scalar
impl Div<Q32> for Vec3Q32 {
    type Output = Self;

    #[inline(always)]
    fn div(self, rhs: Q32) -> Self {
        Vec3Q32::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl Neg for Vec3Q32 {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self {
        Vec3Q32::new(-self.x, -self.y, -self.z)
    }
}
