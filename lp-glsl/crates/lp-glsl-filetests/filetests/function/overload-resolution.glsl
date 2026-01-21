// test run
// target riscv32.fixed32

// ============================================================================
// Overload Resolution: Choosing the best matching overload
// ============================================================================

float test_overload_resolution_exact_match() {
    // Exact match preferred over conversions
    float process(float x) {
        return x * 2.0;
    }

    int process(int x) {
        return x * 3;
    }

    // Exact matches
    return float(process(5.0)) + float(process(5)); // Should be 10.0 + 15 = 25.0
}

// run: test_overload_resolution_exact_match() ~= 25.0

float test_overload_resolution_conversions() {
    // Implicit conversions when exact match not found
    float accept_float(float x) {
        return x;
    }

    float accept_int(int x) {
        return float(x) + 0.5;
    }

    // int to float conversion
    return accept_float(5) + accept_int(3); // 5.0 + (3.0 + 0.5) = 8.5
}

// run: test_overload_resolution_conversions() ~= 8.5

float test_overload_resolution_vector_promotion() {
    // Vector type promotions
    float length_func(vec2 v) {
        return length(v) + 10.0;
    }

    float length_func(vec3 v) {
        return length(v) + 20.0;
    }

    float length_func(vec4 v) {
        return length(v) + 30.0;
    }

    // Test vec3 input
    return length_func(vec3(1.0, 0.0, 0.0)); // Should match vec3 overload
}

// run: test_overload_resolution_vector_promotion() ~= 21.0

float test_overload_resolution_mixed_precision() {
    // Mixed precision handling
    float mix_func(float a, float b) {
        return a + b;
    }

    float mix_func(int a, int b) {
        return float(a + b) + 0.1;
    }

    // Mixed types - should find best match
    return mix_func(1.0, 2) + mix_func(3, 4); // 3.0 + 7.1 = 10.1
}

// run: test_overload_resolution_mixed_precision() ~= 10.1

vec2 test_overload_resolution_vector_construction() {
    // Vector construction overloads
    vec2 make_vec2(float x, float y) {
        return vec2(x, y) * 2.0;
    }

    vec2 make_vec2(vec2 v) {
        return v * 3.0;
    }

    // Should choose vec2(vec2) over vec2(float, float)
    vec2 input = vec2(1.0, 2.0);
    return make_vec2(input); // Should be vec2(3.0, 6.0)
}

// run: test_overload_resolution_vector_construction() ~= vec2(3.0, 6.0)

float test_overload_resolution_matrix_ops() {
    // Matrix operation overloads
    mat2 transform(mat2 m) {
        return m * 2.0;
    }

    mat3 transform(mat3 m) {
        return m * 3.0;
    }

    // Test mat2 input
    mat2 input = mat2(1.0, 0.0, 0.0, 1.0);
    mat2 result = transform(input);
    return result[0][0] + result[1][1]; // Should be 4.0
}

// run: test_overload_resolution_matrix_ops() ~= 4.0

float test_overload_resolution_array_sizes() {
    // Array size overloads
    float sum_elements(float[2] arr) {
        return arr[0] + arr[1] + 1.0;
    }

    float sum_elements(float[3] arr) {
        return arr[0] + arr[1] + arr[2] + 2.0;
    }

    // Test different array sizes
    float[2] arr2 = float[2](1.0, 2.0);
    float[3] arr3 = float[3](1.0, 2.0, 3.0);
    return sum_elements(arr2) + sum_elements(arr3); // 4.0 + 8.0 = 12.0
}

// run: test_overload_resolution_array_sizes() ~= 12.0

bool test_overload_resolution_bool_conversions() {
    // Boolean conversion overloads
    bool check_value(bool b) {
        return b;
    }

    bool check_value(int i) {
        return i != 0;
    }

    bool check_value(float f) {
        return f != 0.0;
    }

    // Test conversions to bool
    return check_value(true) && check_value(1) && check_value(1.0);
}

// run: test_overload_resolution_bool_conversions() == true
