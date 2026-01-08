// test run
// target riscv32.fixed32

// ============================================================================
// Test hue mapping from angle to [0, 1] range
// ============================================================================

// Test angle to hue conversion
float angle_to_hue(float angle) {
    // atan returns [-PI, PI]
    // We want to map this to [0, 1] for hue
    // Normalize: (angle / (2 * PI) + 1.0) * 0.5
    return (angle / (2.0 * 3.14159) + 1.0) * 0.5;
}

// Test atan(0, 1) = 0 (pointing right)
float test_atan_right() {
    return atan(0.0, 1.0);
}

// run: test_atan_right() ~= 0.0

// Test atan(1, 0) = PI/2 (pointing up)
float test_atan_up() {
    return atan(1.0, 0.0);
}

// run: test_atan_up() ~= 1.5707963267948966

// Test atan(0, -1) = PI (pointing left)
float test_atan_left() {
    return atan(0.0, -1.0);
}

// run: test_atan_left() ~= 3.141592653589793

// Test atan(-1, 0) = -PI/2 (pointing down)
float test_atan_down() {
    return atan(-1.0, 0.0);
}

// run: test_atan_down() ~= -1.5707963267948966

// Test hue for right (angle = 0)
float test_hue_right() {
    return angle_to_hue(0.0);
}

// run: test_hue_right() ~= 0.5

// Test hue for up (angle = PI/2)
float test_hue_up() {
    return angle_to_hue(1.5707963267948966);
}

// run: test_hue_up() ~= 0.75

// Test hue for left (angle = PI)
float test_hue_left() {
    return angle_to_hue(3.141592653589793);
}

// run: test_hue_left() ~= 0.75

// Test hue for down (angle = -PI/2)
float test_hue_down() {
    return angle_to_hue(-1.5707963267948966);
}

// run: test_hue_down() ~= 0.25

// Test hue range: what range do we actually get?
float test_hue_min() {
    return angle_to_hue(-3.141592653589793);
}

// run: test_hue_min() ~= 0.0

float test_hue_max() {
    return angle_to_hue(3.141592653589793);
}

// run: test_hue_max() ~= 1.0 (tolerance: 0.25)

