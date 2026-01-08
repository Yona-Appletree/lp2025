// test run
// target riscv32.fixed32

// ============================================================================
// Function Overloading: Same name, different parameter types
// ============================================================================

// Overloaded functions with different parameter types
float test_overload_same_name() {
    // Overloaded add functions
    float add(float a, float b) {
        return a + b;
    }

    int add(int a, int b) {
        return a + b;
    }

    uint add(uint a, uint b) {
        return a + b;
    }

    // Test calling different overloads
    float result = add(1.5, 2.5) + float(add(3, 4)) + float(add(5u, 6u));
    return result;
}

// run: test_overload_same_name() ~= 21.0

float test_overload_vector_types() {
    // Overloaded functions with different vector types
    float length_squared(vec2 v) {
        return dot(v, v);
    }

    float length_squared(vec3 v) {
        return dot(v, v);
    }

    float length_squared(vec4 v) {
        return dot(v, v);
    }

    // Test different vector overloads
    float result = length_squared(vec2(3.0, 4.0)) +  // 25.0
                   length_squared(vec3(1.0, 2.0, 2.0)) +  // 9.0
                   length_squared(vec4(1.0, 1.0, 1.0, 1.0)); // 4.0
    return result;
}

// run: test_overload_vector_types() ~= 38.0

float test_overload_mixed_types() {
    // Overloaded functions mixing scalar and vector types
    vec2 scale(vec2 v, float s) {
        return v * s;
    }

    vec3 scale(vec3 v, float s) {
        return v * s;
    }

    vec4 scale(vec4 v, float s) {
        return v * s;
    }

    // Test scaling different vector types
    float result = scale(vec2(1.0, 2.0), 2.0).x +  // 2.0
                   scale(vec3(1.0, 2.0, 3.0), 2.0).y +  // 4.0
                   scale(vec4(1.0, 2.0, 3.0, 4.0), 2.0).z; // 6.0
    return result;
}

// run: test_overload_mixed_types() ~= 12.0

float test_overload_parameter_count() {
    // Overloaded functions with different parameter counts
    float sum(float a) {
        return a;
    }

    float sum(float a, float b) {
        return a + b;
    }

    float sum(float a, float b, float c) {
        return a + b + c;
    }

    // Test different parameter counts
    float result = sum(1.0) + sum(1.0, 2.0) + sum(1.0, 2.0, 3.0);
    return result;
}

// run: test_overload_parameter_count() ~= 12.0

float test_overload_matrix_types() {
    // Overloaded functions with different matrix types
    float determinant2(mat2 m) {
        return m[0][0] * m[1][1] - m[0][1] * m[1][0];
    }

    float determinant3(mat3 m) {
        return determinant(m);
    }

    // Test matrix determinant overloads
    mat2 m2 = mat2(1.0, 2.0, 3.0, 4.0);
    mat3 m3 = mat3(1.0);
    float result = determinant2(m2) + determinant3(m3);
    return result;
}

// run: test_overload_matrix_types() ~= -1.0

bool test_overload_bool_types() {
    // Overloaded functions with boolean types
    bool is_zero(float x) {
        return x == 0.0;
    }

    bool is_zero(int x) {
        return x == 0;
    }

    bool is_zero(vec2 v) {
        return all(equal(v, vec2(0.0)));
    }

    // Test boolean overloads
    return is_zero(0.0) && is_zero(0) && is_zero(vec2(0.0, 0.0));
}

// run: test_overload_bool_types() == true

float test_overload_array_types() {
    // Overloaded functions with array types
    float sum_array(float[2] arr) {
        return arr[0] + arr[1];
    }

    float sum_array(float[3] arr) {
        return arr[0] + arr[1] + arr[2];
    }

    // Test array overloads
    float[2] arr2 = float[2](1.0, 2.0);
    float[3] arr3 = float[3](1.0, 2.0, 3.0);
    return sum_array(arr2) + sum_array(arr3);
}

// run: test_overload_array_types() ~= 9.0
