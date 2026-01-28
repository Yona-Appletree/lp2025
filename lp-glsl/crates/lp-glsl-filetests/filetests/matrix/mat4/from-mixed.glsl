// test run
// target riscv32.q32

// ============================================================================
// From Mixed: mat4(vec4, vec4, vec4, float, float, float, float) etc. - construct from mixed types
// ============================================================================

mat4 test_mat4_from_vec4_vec4_vec4_vec4() {
    // Constructor mat4(vec4, vec4, vec4, vec4) - each vec4 becomes a column
    return mat4(vec4(1.0, 2.0, 3.0, 4.0), vec4(5.0, 6.0, 7.0, 8.0), vec4(9.0, 10.0, 11.0, 12.0), vec4(13.0, 14.0, 15.0, 16.0));
}

// run: test_mat4_from_vec4_vec4_vec4_vec4() ~= mat4(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0) [expect-fail]

mat4 test_mat4_from_vec4_vec4_vec2_float() {
    return mat4(vec4(1.0, 2.0, 3.0, 4.0), vec4(5.0, 6.0, 7.0, 8.0), vec2(9.0, 10.0), 11.0);
}

// run: test_mat4_from_vec4_vec4_vec2_float() ~= mat4(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 0.0, 0.0, 0.0, 0.0, 0.0) [expect-fail]

mat4 test_mat4_from_multiple_scalars() {
    return mat4(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0);
}

// run: test_mat4_from_multiple_scalars() ~= mat4(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0) [expect-fail]

mat4 test_mat4_from_mixed_expressions() {
    return mat4(vec4(1.0, 2.0, 3.0, 4.0) + vec4(0.5, 0.5, 0.5, 0.5), vec4(5.0, 6.0, 7.0, 8.0) * vec4(2.0, 2.0, 2.0, 2.0), vec2(9.0, 10.0) * vec2(3.0, 3.0), 11.0 * 4.0);
}

// run: test_mat4_from_mixed_expressions() ~= mat4(1.5, 2.5, 3.5, 4.5, 10.0, 12.0, 14.0, 16.0, 27.0, 30.0, 44.0, 0.0, 0.0, 0.0, 0.0, 0.0) [expect-fail]

mat4 test_mat4_from_mixed_variables() {
    vec4 v = vec4(1.0, 2.0, 3.0, 4.0);
    vec4 w = vec4(5.0, 6.0, 7.0, 8.0);
    vec2 x = vec2(9.0, 10.0);
    float a = 11.0;
    return mat4(v, w, x, a);
}

// run: test_mat4_from_mixed_variables() ~= mat4(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 0.0, 0.0, 0.0, 0.0, 0.0) [expect-fail]

mat4 test_mat4_from_mixed_negative() {
    return mat4(vec4(-1.0, -2.0, -3.0, -4.0), vec4(-5.0, -6.0, -7.0, -8.0), vec2(-9.0, -10.0), -11.0);
}

// run: test_mat4_from_mixed_negative() ~= mat4(-1.0, -2.0, -3.0, -4.0, -5.0, -6.0, -7.0, -8.0, -9.0, -10.0, -11.0, 0.0, 0.0, 0.0, 0.0, 0.0) [expect-fail]

mat4 test_mat4_from_mixed_zero() {
    return mat4(vec4(0.0, 0.0, 0.0, 0.0), vec4(0.0, 0.0, 0.0, 0.0), vec2(0.0, 0.0), 0.0);
}

// run: test_mat4_from_mixed_zero() ~= mat4(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0) [expect-fail]

mat4 test_mat4_from_mixed_in_assignment() {
    mat4 result;
    result = mat4(vec4(1.0, 2.0, 3.0, 4.0), vec4(5.0, 6.0, 7.0, 8.0), vec2(9.0, 10.0), 11.0);
    return result;
}

// run: test_mat4_from_mixed_in_assignment() ~= mat4(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 0.0, 0.0, 0.0, 0.0, 0.0) [expect-fail]




