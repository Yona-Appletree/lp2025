// test run
// target riscv32.fixed32

// ============================================================================
// Const Parameters: Read-only parameters
// ============================================================================

float test_param_const_simple() {
    // Const parameter - cannot be modified
    float use_const(const float value) {
        // value = value + 1.0;  // This would be a compile error
        return value * 2.0;
    }

    return use_const(5.0);
}

// run: test_param_const_simple() ~= 10.0

float test_param_const_in_vector() {
    // Const parameter with vector type
    float sum_components(const vec2 v) {
        return v.x + v.y;
    }

    return sum_components(vec2(3.0, 4.0));
}

// run: test_param_const_in_vector() ~= 7.0

int test_param_const_int() {
    // Const parameter with integer type
    int multiply_by_three(const int value) {
        return value * 3;
    }

    return multiply_by_three(7);
}

// run: test_param_const_int() == 21

bool test_param_const_bool() {
    // Const parameter with boolean type
    bool negate(const bool flag) {
        return !flag;
    }

    return negate(true);
}

// run: test_param_const_bool() == false

float test_param_const_in_expression() {
    // Const parameter used in complex expression
    float complex_calc(const float x, const float y) {
        return (x + y) * (x - y);
    }

    return complex_calc(3.0, 4.0);
}

// run: test_param_const_in_expression() ~= -7.0

vec3 test_param_const_vector_ops() {
    // Const parameter with vector operations
    vec3 normalize_components(const vec3 v) {
        return v / length(v);
    }

    vec3 result = normalize_components(vec3(3.0, 4.0, 5.0));
    return result;
}

// run: test_param_const_vector_ops() ~= vec3(0.424264, 0.565685, 0.707107)

float test_param_const_pass_through() {
    // Const parameter passed to another function
    float helper(const float x) {
        return x + 1.0;
    }

    float process(const float value) {
        return helper(value) * 2.0;
    }

    return process(3.0);
}

// run: test_param_const_pass_through() ~= 8.0
