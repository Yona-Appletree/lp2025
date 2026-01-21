// test run
// target riscv32.fixed32

// ============================================================================
// Default Parameter Qualifier: 'in' is the default
// ============================================================================

float test_param_default_explicit_in() {
    // Explicit 'in' qualifier
    float add_explicit(in float a, in float b) {
        return a + b;
    }

    return add_explicit(2.0, 3.0);
}

// run: test_param_default_explicit_in() ~= 5.0

float test_param_default_implicit_in() {
    // Implicit 'in' qualifier (default)
    float add_implicit(float a, float b) {
        return a + b;
    }

    return add_implicit(2.0, 3.0);
}

// run: test_param_default_implicit_in() ~= 5.0

float test_param_default_mixed() {
    // Mix of explicit and implicit in qualifiers
    float process(in float a, float b, in float c) {
        return a + b + c;
    }

    return process(1.0, 2.0, 3.0);
}

// run: test_param_default_mixed() ~= 6.0

float test_param_default_vector() {
    // Default qualifier with vectors
    vec2 combine_vectors(vec2 a, vec2 b) {
        return a + b;
    }

    return length(combine_vectors(vec2(1.0, 2.0), vec2(3.0, 4.0)));
}

// run: test_param_default_vector() ~= 10.0

int test_param_default_int() {
    // Default qualifier with integers
    int multiply(int x, int y) {
        return x * y;
    }

    return multiply(6, 7);
}

// run: test_param_default_int() == 42

bool test_param_default_bool() {
    // Default qualifier with booleans
    bool logical_and(bool a, bool b) {
        return a && b;
    }

    return logical_and(true, true);
}

// run: test_param_default_bool() == true

float test_param_default_modification() {
    // Parameters can be modified inside function (only affects local copy)
    float modify_local(float x) {
        x = x + 10.0; // Modifies local copy only
        return x;
    }

    float original = 5.0;
    float result = modify_local(original);
    return result; // Should be 15.0, original unchanged
}

// run: test_param_default_modification() ~= 15.0

mat2 test_param_default_matrix() {
    // Default qualifier with matrices
    mat2 multiply_matrices(mat2 a, mat2 b) {
        return a * b;
    }

    mat2 m1 = mat2(1.0, 2.0, 3.0, 4.0);
    mat2 m2 = mat2(2.0);
    mat2 result = multiply_matrices(m1, m2);
    return result;
}

// run: test_param_default_matrix() ~= mat2(2.0, 4.0, 6.0, 8.0)

float test_param_default_array() {
    // Default qualifier with arrays
    float sum_elements(float[3] arr) {
        return arr[0] + arr[1] + arr[2];
    }

    float[3] data = float[3](1.0, 2.0, 3.0);
    return sum_elements(data);
}

// run: test_param_default_array() ~= 6.0

struct Point {
    float x, y;
};

Point test_param_default_struct() {
    // Default qualifier with structs
    Point move_point(Point p, float dx, float dy) {
        return Point(p.x + dx, p.y + dy);
    }

    Point p = Point(1.0, 2.0);
    return move_point(p, 3.0, 4.0);
}

// run: test_param_default_struct() ~= Point(4.0, 6.0)
