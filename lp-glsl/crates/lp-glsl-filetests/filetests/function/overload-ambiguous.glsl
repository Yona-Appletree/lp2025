// test run
// target riscv32.fixed32

// ============================================================================
// Ambiguous Overloads: Cases that should produce compile errors
// Note: These tests are expected to FAIL compilation due to ambiguous overloads
// They are included to test that the compiler properly detects ambiguity
// ============================================================================

// Ambiguous overloads - same parameter types, different return types
// This should be a compile error since overloading is based on parameters, not return type

/*
float test_overload_ambiguous_return_type() {
    // Same parameters, different return types - INVALID
    float func(int x) {
        return float(x);
    }

    int func(int x) {
        return x;
    }

    // This call would be ambiguous
    return func(5); // Error: ambiguous call
}

// run: test_overload_ambiguous_return_type() ~= 5.0
*/

float test_overload_ambiguous_qualifiers() {
    // Same parameter types with different qualifiers - INVALID
    float func(in int x) {
        return float(x);
    }

    float func(out int x) {  // Different qualifier
        x = 10;
        return 5.0;
    }

    // This should be an error - same base types, different qualifiers
    return func(5); // Should be compile error
}

// run: test_overload_ambiguous_qualifiers() ~= 5.0

/*
float test_overload_ambiguous_conversions() {
    // Multiple equally good conversions
    float func(float x, int y) {
        return x + float(y);
    }

    float func(int x, float y) {
        return float(x) + y;
    }

    // Call with (int, int) - both overloads require one conversion each
    // This should be ambiguous
    return func(1, 2); // Error: ambiguous - both equally good
}

// run: test_overload_ambiguous_conversions() ~= 3.0
*/

float test_overload_valid_resolution() {
    // Valid case for comparison - exact match vs conversion
    float func(float x, float y) {
        return x + y;
    }

    float func(float x, int y) {
        return x + float(y) + 0.5; // Mark this one
    }

    // Should prefer exact match
    return func(1.0, 2.0); // Should call first overload: 3.0
}

// run: test_overload_valid_resolution() ~= 3.0

/*
float test_overload_ambiguous_array_sizes() {
    // Different array sizes - should be distinct, not ambiguous
    float sum(float[2] arr) {
        return arr[0] + arr[1];
    }

    float sum(float[3] arr) {
        return arr[0] + arr[1] + arr[2];
    }

    // These should be valid overloads since array sizes are part of the type
    float[2] arr2 = float[2](1.0, 2.0);
    float[3] arr3 = float[3](1.0, 2.0, 3.0);
    return sum(arr2) + sum(arr3); // 3.0 + 6.0 = 9.0
}

// run: test_overload_ambiguous_array_sizes() ~= 9.0
*/

float test_overload_ambiguous_vector_sizes() {
    // Different vector dimensions should be valid overloads
    float get_x(vec2 v) {
        return v.x + 10.0;
    }

    float get_x(vec3 v) {
        return v.x + 20.0;
    }

    float get_x(vec4 v) {
        return v.x + 30.0;
    }

    // Should choose vec3 overload
    return get_x(vec3(5.0, 0.0, 0.0)); // Should be 25.0
}

// run: test_overload_ambiguous_vector_sizes() ~= 25.0

/*
float test_overload_ambiguous_promotions() {
    // Ambiguous due to multiple possible promotions
    float process(int x) {
        return float(x);
    }

    float process(uint x) {
        return float(x) + 0.1;
    }

    // int literal 5 could match both int and uint
    // This might be ambiguous depending on language rules
    return process(5); // Potentially ambiguous
}

// run: test_overload_ambiguous_promotions() ~= 5.0
*/
