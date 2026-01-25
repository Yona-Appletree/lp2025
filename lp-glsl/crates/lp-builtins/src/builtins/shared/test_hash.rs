//! Deterministic hash functions for testing.
//!
//! These functions provide simple, deterministic hash values for reproducible testing.
//! They are only available when the `test_hash_fixed` feature is enabled.

/// Deterministic hash function for 1D coordinates (testing only).
///
/// Returns a simple deterministic value based on inputs.
/// Uses multiplicative hashing with large primes for good distribution.
#[cfg(feature = "test_hash_fixed")]
pub fn hash_1(x: u32, seed: u32) -> u32 {
    // Simple deterministic hash: x * prime1 + seed * prime2
    // Primes chosen for good distribution
    x.wrapping_mul(2654435761)
        .wrapping_add(seed.wrapping_mul(2246822519))
}

/// Deterministic hash function for 2D coordinates (testing only).
#[cfg(feature = "test_hash_fixed")]
pub fn hash_2(x: u32, y: u32, seed: u32) -> u32 {
    // Combine coordinates non-commutatively
    let combined = x
        .wrapping_mul(2654435761)
        .wrapping_add(y.wrapping_mul(2246822519));
    hash_1(combined, seed)
}

/// Deterministic hash function for 3D coordinates (testing only).
#[cfg(feature = "test_hash_fixed")]
pub fn hash_3(x: u32, y: u32, z: u32, seed: u32) -> u32 {
    // Combine coordinates non-commutatively
    let combined = x
        .wrapping_mul(2654435761)
        .wrapping_add(y.wrapping_mul(2246822519))
        .wrapping_add(z.wrapping_mul(3266489917));
    hash_1(combined, seed)
}
