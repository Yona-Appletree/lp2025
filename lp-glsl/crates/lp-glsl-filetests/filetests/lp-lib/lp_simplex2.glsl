// test run
// target riscv32.fixed32

// ============================================================================
// lp_simplex2(): 2D Simplex noise function
// ============================================================================

float test_lp_simplex2_basic() {
    // Test basic 2D simplex noise - should be in [-1, 1] range
    vec2 p = vec2(0.5, 0.5);
    uint seed = 0u;
    float n = lp_simplex2(p, seed);
    return (n >= -1.0 && n <= 1.0) ? 1.0 : 0.0;
}

// run: test_lp_simplex2_basic() == 1.0

float test_lp_simplex2_origin() {
    // Test at origin - should be in [-1, 1] range
    vec2 p = vec2(0.0, 0.0);
    uint seed = 0u;
    float n = lp_simplex2(p, seed);
    return (n >= -1.0 && n <= 1.0) ? 1.0 : 0.0;
}

// run: test_lp_simplex2_origin() == 1.0

float test_lp_simplex2_deterministic() {
    // Same inputs should produce same output
    vec2 p = vec2(0.5, 0.5);
    float n1 = lp_simplex2(p, 0u);
    float n2 = lp_simplex2(p, 0u);
    return abs(n1 - n2);
}

// run: test_lp_simplex2_deterministic() ~= 0.0

float test_lp_simplex2_different_seeds() {
    // Different seeds should produce different outputs
    vec2 p = vec2(0.5, 0.5);
    float n1 = lp_simplex2(p, 0u);
    float n2 = lp_simplex2(p, 1u);
    return abs(n1 - n2) > 0.01 ? 1.0 : 0.0;
}

// run: test_lp_simplex2_different_seeds() == 1.0

float test_lp_simplex2_different_positions() {
    // Different positions should produce different outputs (with high probability)
    float n1 = lp_simplex2(vec2(0.0, 0.0), 0u);
    float n2 = lp_simplex2(vec2(1.0, 1.0), 0u);
    return abs(n1 - n2) > 0.01 ? 1.0 : 0.0;
}

// run: test_lp_simplex2_different_positions() == 1.0

float test_lp_simplex2_range() {
    // Test multiple values to ensure they're in valid range
    float n1 = lp_simplex2(vec2(0.0, 0.0), 0u);
    float n2 = lp_simplex2(vec2(1.0, 0.0), 0u);
    float n3 = lp_simplex2(vec2(0.0, 1.0), 0u);
    float n4 = lp_simplex2(vec2(10.0, 10.0), 0u);
    
    // All should be in [-1, 1] range
    bool valid = n1 >= -1.0 && n1 <= 1.0 &&
                 n2 >= -1.0 && n2 <= 1.0 &&
                 n3 >= -1.0 && n3 <= 1.0 &&
                 n4 >= -1.0 && n4 <= 1.0;
    return valid ? 1.0 : 0.0;
}

// run: test_lp_simplex2_range() == 1.0
