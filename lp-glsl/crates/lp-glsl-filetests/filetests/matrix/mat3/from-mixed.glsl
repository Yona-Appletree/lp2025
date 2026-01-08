// test run
// target riscv32.fixed32

// ============================================================================
// From Mixed: mat3(vec3, vec3, float, float, float) etc. - construct from mixed types
// ============================================================================

mat3 test_mat3_from_vec3_vec3_vec3() {
    // Constructor mat3(vec3, vec3, vec3) - each vec3 becomes a column
    return mat3(vec3(1.0, 2.0, 3.0), vec3(4.0, 5.0, 6.0), vec3(7.0, 8.0, 9.0));
}

// run: test_mat3_from_vec3_vec3_vec3() ~= mat3(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0)

mat3 test_mat3_from_vec3_vec2_float() {
    return mat3(vec3(1.0, 2.0, 3.0), vec2(4.0, 5.0), 6.0);
}

// run: test_mat3_from_vec3_vec2_float() ~= mat3(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 0.0, 0.0, 0.0)

mat3 test_mat3_from_multiple_scalars() {
    return mat3(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
}

// run: test_mat3_from_multiple_scalars() ~= mat3(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0)

mat3 test_mat3_from_mixed_expressions() {
    return mat3(vec3(1.0, 2.0, 3.0) + vec3(0.5, 0.5, 0.5), vec2(4.0, 5.0) * vec2(2.0, 2.0), 6.0 * 3.0);
}

// run: test_mat3_from_mixed_expressions() ~= mat3(1.5, 2.5, 3.5, 8.0, 10.0, 18.0, 0.0, 0.0, 0.0)

mat3 test_mat3_from_mixed_variables() {
    vec3 v = vec3(1.0, 2.0, 3.0);
    vec2 w = vec2(4.0, 5.0);
    float a = 6.0;
    return mat3(v, w, a);
}

// run: test_mat3_from_mixed_variables() ~= mat3(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 0.0, 0.0, 0.0)

mat3 test_mat3_from_mixed_negative() {
    return mat3(vec3(-1.0, -2.0, -3.0), vec2(-4.0, -5.0), -6.0);
}

// run: test_mat3_from_mixed_negative() ~= mat3(-1.0, -2.0, -3.0, -4.0, -5.0, -6.0, 0.0, 0.0, 0.0)

mat3 test_mat3_from_mixed_zero() {
    return mat3(vec3(0.0, 0.0, 0.0), vec2(0.0, 0.0), 0.0);
}

// run: test_mat3_from_mixed_zero() ~= mat3(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)

mat3 test_mat3_from_mixed_in_assignment() {
    mat3 result;
    result = mat3(vec3(1.0, 2.0, 3.0), vec2(4.0, 5.0), 6.0);
    return result;
}

// run: test_mat3_from_mixed_in_assignment() ~= mat3(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 0.0, 0.0, 0.0)




