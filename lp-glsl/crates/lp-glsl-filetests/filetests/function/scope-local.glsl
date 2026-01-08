// test run
// target riscv32.fixed32

// ============================================================================
// Local Variable Scope: Variables declared in functions
// ============================================================================

float global_value = 100.0;

float test_scope_local_simple() {
    // Local variables are scoped to function
    float local_func() {
        float local_var = 42.0;
        return local_var;
    }

    return local_func();
}

// run: test_scope_local_simple() ~= 42.0

float test_scope_local_shadow_global() {
    // Local variables shadow globals
    float access_global() {
        float global_value = 200.0; // Shadows global
        return global_value;
    }

    return access_global();
}

// run: test_scope_local_shadow_global() ~= 200.0

float test_scope_local_multiple() {
    // Multiple local variables
    float process_locals() {
        float a = 1.0;
        float b = 2.0;
        float c = a + b;
        return c;
    }

    return process_locals();
}

// run: test_scope_local_multiple() ~= 3.0

float test_scope_local_in_loop() {
    // Local variables in loops
    float sum_loop(int n) {
        float sum = 0.0;
        for (int i = 0; i < n; i++) {
            float local_i = float(i);
            sum = sum + local_i;
        }
        return sum;
    }

    return sum_loop(5);
}

// run: test_scope_local_in_loop() ~= 10.0

float test_scope_local_nested() {
    // Nested function scopes
    float outer_func() {
        float outer_var = 10.0;

        float inner_func() {
            float inner_var = 20.0;
            return outer_var + inner_var; // Can access outer
        }

        return inner_func();
    }

    return outer_func();
}

// run: test_scope_local_nested() ~= 30.0

float test_scope_local_parameters() {
    // Parameters are also local to function
    float use_params(float param1, float param2) {
        float local_calc = param1 * 2.0 + param2 * 3.0;
        return local_calc;
    }

    return use_params(2.0, 3.0);
}

// run: test_scope_local_parameters() ~= 13.0

float test_scope_local_types() {
    // Different types of local variables
    float mixed_types() {
        int int_var = 5;
        float float_var = 3.14;
        bool bool_var = true;
        vec2 vec_var = vec2(1.0, 2.0);

        return float(int_var) + float_var + (bool_var ? 1.0 : 0.0) + vec_var.x + vec_var.y;
    }

    return mixed_types();
}

// run: test_scope_local_types() ~= 12.14

float test_scope_local_arrays() {
    // Local arrays
    float sum_local_array() {
        float[3] local_arr = float[3](1.0, 2.0, 3.0);
        return local_arr[0] + local_arr[1] + local_arr[2];
    }

    return sum_local_array();
}

// run: test_scope_local_arrays() ~= 6.0

struct LocalStruct {
    float x, y;
};

LocalStruct test_scope_local_struct() {
    // Local structs
    LocalStruct create_local_struct() {
        LocalStruct s = LocalStruct(5.0, 10.0);
        return s;
    }

    return create_local_struct();
}

// run: test_scope_local_struct() ~= LocalStruct(5.0, 10.0)

float test_scope_local_modification() {
    // Local variables can be modified
    float modify_local() {
        float value = 5.0;
        value = value * 2.0;
        value = value + 3.0;
        return value;
    }

    return modify_local();
}

// run: test_scope_local_modification() ~= 13.0
