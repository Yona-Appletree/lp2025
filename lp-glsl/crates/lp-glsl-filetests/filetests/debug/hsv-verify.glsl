// test run
// target riscv32.fixed32

// ============================================================================
// Verify HSV to RGB conversion step by step
// ============================================================================

// HSV to RGB conversion function
vec3 hsv_to_rgb(float h, float s, float v) {
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

// Test intermediate values for h=0.5, s=0.5, v=0.75
float test_c_value() {
    float h = 0.5;
    float s = 0.5;
    float v = 0.75;
    float c = v * s;
    return c;
}

// run: test_c_value() ~= 0.375

float test_x_value() {
    float h = 0.5;
    float s = 0.5;
    float v = 0.75;
    float c = v * s; // 0.375
    float h_times_6 = h * 6.0; // 3.0
    float mod_result = mod(h_times_6, 2.0); // mod(3.0, 2.0) = 1.0
    float abs_value = abs(mod_result - 1.0); // abs(1.0 - 1.0) = 0.0
    float x = c * (1.0 - abs_value); // 0.375 * (1.0 - 0.0) = 0.375
    return x;
}

// run: test_x_value() ~= 0.375

float test_m_value() {
    float h = 0.5;
    float s = 0.5;
    float v = 0.75;
    float c = v * s; // 0.375
    float m = v - c; // 0.75 - 0.375 = 0.375
    return m;
}

// run: test_m_value() ~= 0.375

// Test which branch we take for h=0.5
float test_hue_branch() {
    float h = 0.5;
    // h < 1/6? 0.5 < 0.166? No
    // h < 2/6? 0.5 < 0.333? No
    // h < 3/6? 0.5 < 0.5? No (equal)
    // h < 4/6? 0.5 < 0.666? Yes -> branch 4 (cyan)
    // So: rgb = vec3(m, m + x, v) = vec3(0.375, 0.375 + 0.375, 0.75) = vec3(0.375, 0.75, 0.75)
    if (h < 1.0/6.0) {
        return 1.0; // red branch
    } else if (h < 2.0/6.0) {
        return 2.0; // yellow branch
    } else if (h < 3.0/6.0) {
        return 3.0; // green branch
    } else if (h < 4.0/6.0) {
        return 4.0; // cyan branch
    } else if (h < 5.0/6.0) {
        return 5.0; // blue branch
    } else {
        return 6.0; // magenta branch
    }
}

// run: test_hue_branch() ~= 4.0

// Test the actual RGB result
vec3 test_hsv_result() {
    float h = 0.5;
    float s = 0.5;
    float v = 0.75;
    return hsv_to_rgb(h, s, v);
}

// run: test_hsv_result() ~= vec3(0.375, 0.75, 0.75)

