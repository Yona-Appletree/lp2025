// test run
// target riscv32.fixed32

// ============================================================================
// Debug test for rotating color wheel shader
// Tests the combination of atan, length, and HSV conversion used in the
// default project shader
// ============================================================================

// Test atan with vec2 (two-argument version)
float test_atan_vec2_two_arg() {
    vec2 dir = vec2(1.0, 0.0); // Pointing right (0 degrees)
    return atan(dir.y, dir.x);
}

// run: test_atan_vec2_two_arg() ~= 0.0

float test_atan_vec2_two_arg_45deg() {
    vec2 dir = vec2(1.0, 1.0); // 45 degrees
    return atan(dir.y, dir.x);
}

// run: test_atan_vec2_two_arg_45deg() ~= 0.7853981633974483

float test_atan_vec2_two_arg_90deg() {
    vec2 dir = vec2(0.0, 1.0); // Pointing up (90 degrees)
    return atan(dir.y, dir.x);
}

// run: test_atan_vec2_two_arg_90deg() ~= 1.5707963267948966

// Test length with vec2
float test_length_vec2() {
    vec2 dir = vec2(3.0, 4.0);
    return length(dir);
}

// run: test_length_vec2() ~= 5.0

float test_length_vec2_normalized() {
    vec2 dir = vec2(0.6, 0.8);
    return length(dir);
}

// run: test_length_vec2_normalized() ~= 1.0

// Test the combination: atan + time rotation
float test_atan_with_time_rotation() {
    vec2 dir = vec2(1.0, 0.0);
    float angle = atan(dir.y, dir.x);
    float time = 1.0; // 1 second
    angle = angle + time * 0.5; // Rotate by 0.5 radians
    return angle;
}

// run: test_atan_with_time_rotation() ~= 0.5

// Test normalized angle to [0, 1] for hue
float test_angle_to_hue() {
    vec2 dir = vec2(1.0, 0.0);
    float angle = atan(dir.y, dir.x);
    // Normalize angle to [0, 1] for hue
    float hue = (angle / (2.0 * 3.14159) + 1.0) * 0.5;
    return hue;
}

// run: test_angle_to_hue() ~= 0.5

// Test distance calculation (length normalized by output size)
float test_distance_normalized() {
    vec2 dir = vec2(32.0, 0.0); // 32 pixels from center
    vec2 outputSize = vec2(64.0, 64.0);
    float dist = length(dir) / (min(outputSize.x, outputSize.y) * 0.5);
    return dist;
}

// run: test_distance_normalized() ~= 1.0

float test_distance_normalized_center() {
    vec2 dir = vec2(0.0, 0.0); // At center
    vec2 outputSize = vec2(64.0, 64.0);
    float dist = length(dir) / (min(outputSize.x, outputSize.y) * 0.5);
    return dist;
}

// run: test_distance_normalized_center() ~= 0.0

// Test the full HSV conversion math
float test_hsv_saturation_calc() {
    float dist = 0.5; // Middle distance
    float c = 1.0 - abs(dist - 0.5) * 2.0; // Saturation based on distance
    return c;
}

// run: test_hsv_saturation_calc() ~= 1.0

float test_hsv_saturation_calc_edge() {
    float dist = 1.0; // Edge distance
    float c = 1.0 - abs(dist - 0.5) * 2.0; // Saturation based on distance
    return c;
}

// run: test_hsv_saturation_calc_edge() ~= 0.0

// Test mod function used in HSV conversion
float test_mod_hue_conversion() {
    float hue = 0.5; // Normalized hue [0, 1]
    float x = 1.0 - abs(mod(hue * 6.0, 2.0) - 1.0);
    return x;
}

// run: test_mod_hue_conversion() ~= 1.0

// Test the full pipeline: atan + length + HSV math
vec4 test_full_color_wheel_pixel() {
    vec2 fragCoord = vec2(32.0, 0.0); // Right edge, center Y
    vec2 outputSize = vec2(64.0, 64.0);
    float time = 0.0;
    
    // Center of texture
    vec2 center = outputSize * 0.5;
    
    // Direction from center to fragment
    vec2 dir = fragCoord - center;
    
    // Calculate angle
    float angle = atan(dir.y, dir.x);
    
    // Rotate angle with time
    angle = angle + time * 0.5;
    
    // Normalize angle to [0, 1] for hue
    float hue = (angle / (2.0 * 3.14159) + 1.0) * 0.5;
    
    // Distance from center (normalized)
    float dist = length(dir) / (min(outputSize.x, outputSize.y) * 0.5);
    
    // HSV to RGB conversion (simplified - just return hue as red for testing)
    // In the actual shader, this would do full HSV->RGB conversion
    float r = hue;
    float g = dist;
    float b = 0.5;
    
    return vec4(r, g, b, 1.0);
}

// run: test_full_color_wheel_pixel() ~= vec4(0.36909485, 1.0, 0.5, 1.0)

