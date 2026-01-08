// test run
// target riscv32.fixed32

// ============================================================================
// Debug test for HSV to RGB conversion
// Tests the HSV to RGB conversion used in the rotating color wheel shader
// ============================================================================

// HSV to RGB conversion function
vec3 hsv_to_rgb(float h, float s, float v) {
    // h in [0, 1], s in [0, 1], v in [0, 1]
    float c = v * s;
    float x = c * (1.0 - abs(mod(h * 6.0, 2.0) - 1.0));
    float m = v - c;
    
    vec3 rgb;
    if (h < 1.0/6.0) {
        rgb = vec3(v, m + x, m);
    } else if (h < 2.0/6.0) {
        rgb = vec3(m + x, v, m);
    } else if (h < 3.0/6.0) {
        rgb = vec3(m, v, m + x);
    } else if (h < 4.0/6.0) {
        rgb = vec3(m, m + x, v);
    } else if (h < 5.0/6.0) {
        rgb = vec3(m + x, m, v);
    } else {
        rgb = vec3(v, m, m + x);
    }
    
    return rgb;
}

// Test pure red (hue = 0)
vec3 test_hsv_red() {
    return hsv_to_rgb(0.0, 1.0, 1.0);
}

// run: test_hsv_red() ~= vec3(1.0, 0.0, 0.0)

// Test pure green (hue = 1/3)
vec3 test_hsv_green() {
    return hsv_to_rgb(1.0/3.0, 1.0, 1.0);
}

// run: test_hsv_green() ~= vec3(0.0, 1.0, 0.0)

// Test pure blue (hue = 2/3)
vec3 test_hsv_blue() {
    return hsv_to_rgb(2.0/3.0, 1.0, 1.0);
}

// run: test_hsv_blue() ~= vec3(0.0, 0.0, 1.0)

// Test yellow (hue = 1/6, between red and green)
vec3 test_hsv_yellow() {
    return hsv_to_rgb(1.0/6.0, 1.0, 1.0);
}

// run: test_hsv_yellow() ~= vec3(1.0, 1.0, 0.0)

// Test cyan (hue = 1/2, between green and blue)
vec3 test_hsv_cyan() {
    return hsv_to_rgb(1.0/2.0, 1.0, 1.0);
}

// run: test_hsv_cyan() ~= vec3(0.0, 1.0, 1.0)

// Test magenta (hue = 5/6, between blue and red)
vec3 test_hsv_magenta() {
    return hsv_to_rgb(5.0/6.0, 1.0, 1.0);
}

// run: test_hsv_magenta() ~= vec3(1.0, 0.0, 1.0)

// Test white (saturation = 0, any hue)
vec3 test_hsv_white() {
    return hsv_to_rgb(0.0, 0.0, 1.0);
}

// run: test_hsv_white() ~= vec3(1.0, 1.0, 1.0)

// Test black (value = 0, any hue/saturation)
vec3 test_hsv_black() {
    return hsv_to_rgb(0.0, 1.0, 0.0);
}

// run: test_hsv_black() ~= vec3(0.0, 0.0, 0.0)

// Test grey (saturation = 0, value = 0.5)
vec3 test_hsv_grey() {
    return hsv_to_rgb(0.0, 0.0, 0.5);
}

// run: test_hsv_grey() ~= vec3(0.5, 0.5, 0.5)

// Test with reduced saturation (should be less vibrant)
vec3 test_hsv_low_saturation() {
    return hsv_to_rgb(0.0, 0.5, 1.0); // Red with 50% saturation
}

// run: test_hsv_low_saturation() ~= vec3(1.0, 0.5, 0.5)

// Test with reduced value (should be darker)
vec3 test_hsv_low_value() {
    return hsv_to_rgb(0.0, 1.0, 0.5); // Red with 50% brightness
}

// run: test_hsv_low_value() ~= vec3(0.5, 0.0, 0.0)

// Test a full color wheel pixel calculation (like in the shader)
vec4 test_color_wheel_pixel() {
    // Simulate a pixel at angle 0 (right side), distance 0.5 from center
    float angle = 0.0; // Pointing right
    float hue = (angle / (2.0 * 3.14159) + 1.0) * 0.5; // Should be 0.5
    float dist = 0.5;
    float saturation = 1.0 - dist; // 0.5
    float value = 1.0 - dist * 0.5; // 0.75
    
    vec3 rgb = hsv_to_rgb(hue, saturation, value);
    return vec4(rgb, 1.0);
}

// run: test_color_wheel_pixel() ~= vec4(0.375, 0.75, 0.75, 1.0)

