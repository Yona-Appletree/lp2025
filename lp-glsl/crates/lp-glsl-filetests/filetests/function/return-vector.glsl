// test run
// target riscv32.fixed32

// ============================================================================
// Vector Return Types: vec2, vec3, vec4, ivec2, ivec3, ivec4, etc.
// ============================================================================

vec2 test_return_vec2_simple() {
    // Return vec2 value
    vec2 get_vec2() {
        return vec2(1.0, 2.0);
    }

    return get_vec2();
}

// run: test_return_vec2_simple() ~= vec2(1.0, 2.0)

vec3 test_return_vec3_simple() {
    // Return vec3 value
    vec3 get_vec3() {
        return vec3(1.0, 2.0, 3.0);
    }

    return get_vec3();
}

// run: test_return_vec3_simple() ~= vec3(1.0, 2.0, 3.0)

vec4 test_return_vec4_simple() {
    // Return vec4 value
    vec4 get_vec4() {
        return vec4(1.0, 2.0, 3.0, 4.0);
    }

    return get_vec4();
}

// run: test_return_vec4_simple() ~= vec4(1.0, 2.0, 3.0, 4.0)

ivec2 test_return_ivec2_simple() {
    // Return ivec2 value
    ivec2 get_ivec2() {
        return ivec2(10, 20);
    }

    return get_ivec2();
}

// run: test_return_ivec2_simple() == ivec2(10, 20)

ivec3 test_return_ivec3_simple() {
    // Return ivec3 value
    ivec3 get_ivec3() {
        return ivec3(1, 2, 3);
    }

    return get_ivec3();
}

// run: test_return_ivec3_simple() == ivec3(1, 2, 3)

uvec2 test_return_uvec2_simple() {
    // Return uvec2 value
    uvec2 get_uvec2() {
        return uvec2(100u, 200u);
    }

    return get_uvec2();
}

// run: test_return_uvec2_simple() == uvec2(100u, 200u)

bvec2 test_return_bvec2_simple() {
    // Return bvec2 value
    bvec2 get_bvec2() {
        return bvec2(true, false);
    }

    return get_bvec2();
}

// run: test_return_bvec2_simple() == bvec2(true, false)

vec2 test_return_vec2_calculation() {
    // Return result of vector calculation
    vec2 add_vectors(vec2 a, vec2 b) {
        return a + b;
    }

    return add_vectors(vec2(1.0, 2.0), vec2(3.0, 4.0));
}

// run: test_return_vec2_calculation() ~= vec2(4.0, 6.0)

vec3 test_return_vec3_normalize() {
    // Return normalized vector
    vec3 get_normal() {
        vec3 v = vec3(3.0, 4.0, 5.0);
        return normalize(v);
    }

    return get_normal();
}

// run: test_return_vec3_normalize() ~= vec3(0.424264, 0.565685, 0.707107)

vec4 test_return_vec4_constructor() {
    // Return vec4 constructed from components
    vec4 build_color(float r, float g, float b, float a) {
        return vec4(r, g, b, a);
    }

    return build_color(1.0, 0.5, 0.0, 1.0);
}

// run: test_return_vec4_constructor() ~= vec4(1.0, 0.5, 0.0, 1.0)
