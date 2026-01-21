// test run
// target riscv32.fixed32

// ============================================================================
// In Parameters: Default parameter qualifier (copy in only)
// ============================================================================

float test_param_in_explicit() {
    // Explicit 'in' qualifier
    float add_in(in float a, in float b) {
        return a + b;
    }

    return add_in(3.0, 4.0);
}

// run: test_param_in_explicit() ~= 7.0

float test_param_in_implicit() {
    // Implicit 'in' qualifier (default)
    float multiply(float a, float b) {
        return a * b;
    }

    return multiply(3.0, 4.0);
}

// run: test_param_in_implicit() ~= 12.0

float test_param_in_modify_local() {
    // In parameters can be modified inside function (affects only local copy)
    float modify_and_return(in float x) {
        x = x + 1.0;  // This modifies local copy only
        return x;
    }

    return modify_and_return(5.0);
}

// run: test_param_in_modify_local() ~= 6.0

vec2 test_param_in_vector() {
    // In parameters with vector types
    vec2 add_vectors(in vec2 a, in vec2 b) {
        return a + b;
    }

    return add_vectors(vec2(1.0, 2.0), vec2(3.0, 4.0));
}

// run: test_param_in_vector() ~= vec2(4.0, 6.0)

int test_param_in_int() {
    // In parameters with integer types
    int add_ints(in int a, in int b) {
        return a + b;
    }

    return add_ints(10, 20);
}

// run: test_param_in_int() == 30

uint test_param_in_uint() {
    // In parameters with unsigned integer types
    uint add_uints(in uint a, in uint b) {
        return a + b;
    }

    return add_uints(100u, 200u);
}

// run: test_param_in_uint() == 300u

bool test_param_in_bool() {
    // In parameters with boolean types
    bool and_bools(in bool a, in bool b) {
        return a && b;
    }

    return and_bools(true, false);
}

// run: test_param_in_bool() == false

vec3 test_param_in_modify_components() {
    // Modify individual components of in parameter
    vec3 process_vector(in vec3 v) {
        v.x = v.x * 2.0;
        v.y = v.y + 1.0;
        v.z = v.z - 0.5;
        return v;
    }

    return process_vector(vec3(1.0, 2.0, 3.0));
}

// run: test_param_in_modify_components() ~= vec3(2.0, 3.0, 2.5)
