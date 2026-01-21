// Test hue wrapping when time > 1
// This tests the issue where hue goes outside [0, 1] range when time gets large

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

vec4 main(vec2 fragCoord, vec2 outputSize, float time) {
    // Simulate the angle calculation from the shader
    // For a pixel at (32, 0) relative to center (32, 32), dir = (0, -32)
    // atan(-32, 0) = -PI/2
    vec2 center = outputSize * 0.5;
    vec2 dir = fragCoord - center;
    float angle = atan(dir.y, dir.x);
    
    // Rotate angle with time
    angle = angle + time * 3.14159;
    
    // Normalize angle to [0, 1] for hue - THIS IS THE PROBLEM
    // When time > 1, this can go outside [0, 1]
    float hue = (angle + 3.14159) / (2.0 * 3.14159);
    
    // Test: hue should be wrapped to [0, 1] using mod or fract
    float hue_wrapped = mod(hue, 1.0);
    
    // Use wrapped hue for color calculation
    vec3 rgb = hsv_to_rgb(hue_wrapped, 1.0, 1.0);
    
    return vec4(rgb, 1.0);
}

// Test cases
test test_hue_at_time_0() {
    vec2 fragCoord = vec2(64.0, 32.0); // Right side of center
    vec2 outputSize = vec2(64.0, 64.0);
    float time = 0.0;
    
    vec4 result = main(fragCoord, outputSize, time);
    
    // At (64, 32) relative to center (32, 32), dir = (32, 0)
    // atan(0, 32) = 0
    // angle = 0 + 0 * PI = 0
    // hue = (0 + PI) / (2*PI) = PI/(2*PI) = 0.5
    // hue_wrapped = mod(0.5, 1.0) = 0.5
    // HSV(0.5, 1.0, 1.0) should be cyan
    expect(result).to_be_close_to(vec4(0.0, 1.0, 1.0, 1.0), 0.01);
}

test test_hue_at_time_1() {
    vec2 fragCoord = vec2(64.0, 32.0);
    vec2 outputSize = vec2(64.0, 64.0);
    float time = 1.0;
    
    vec4 result = main(fragCoord, outputSize, time);
    
    // angle = 0 + 1 * PI = PI
    // hue = (PI + PI) / (2*PI) = 2*PI/(2*PI) = 1.0
    // hue_wrapped = mod(1.0, 1.0) = 0.0 (should wrap to 0)
    // HSV(0.0, 1.0, 1.0) should be red
    expect(result).to_be_close_to(vec4(1.0, 0.0, 0.0, 1.0), 0.01);
}

test test_hue_at_time_2() {
    vec2 fragCoord = vec2(64.0, 32.0);
    vec2 outputSize = vec2(64.0, 64.0);
    float time = 2.0;
    
    vec4 result = main(fragCoord, outputSize, time);
    
    // angle = 0 + 2 * PI = 2*PI
    // hue = (2*PI + PI) / (2*PI) = 3*PI/(2*PI) = 1.5
    // hue_wrapped = mod(1.5, 1.0) = 0.5
    // HSV(0.5, 1.0, 1.0) should be cyan again
    expect(result).to_be_close_to(vec4(0.0, 1.0, 1.0, 1.0), 0.01);
}

test test_hue_at_time_0_5() {
    vec2 fragCoord = vec2(64.0, 32.0);
    vec2 outputSize = vec2(64.0, 64.0);
    float time = 0.5;
    
    vec4 result = main(fragCoord, outputSize, time);
    
    // angle = 0 + 0.5 * PI = PI/2
    // hue = (PI/2 + PI) / (2*PI) = (3*PI/2) / (2*PI) = 0.75
    // hue_wrapped = mod(0.75, 1.0) = 0.75
    // HSV(0.75, 1.0, 1.0) should be blue
    expect(result).to_be_close_to(vec4(0.0, 0.0, 1.0, 1.0), 0.01);
}

test test_hue_wrapping_verification() {
    // Test that mod correctly wraps values > 1
    vec2 fragCoord = vec2(64.0, 32.0);
    vec2 outputSize = vec2(64.0, 64.0);
    float time = 3.0;
    
    vec4 result = main(fragCoord, outputSize, time);
    
    // angle = 0 + 3 * PI = 3*PI
    // hue = (3*PI + PI) / (2*PI) = 4*PI/(2*PI) = 2.0
    // hue_wrapped = mod(2.0, 1.0) = 0.0
    // HSV(0.0, 1.0, 1.0) should be red
    expect(result).to_be_close_to(vec4(1.0, 0.0, 0.0, 1.0), 0.01);
}

