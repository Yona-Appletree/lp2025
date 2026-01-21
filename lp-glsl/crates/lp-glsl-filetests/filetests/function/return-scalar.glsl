// test run
// target riscv32.fixed32

// ============================================================================
// Scalar Return Types: float, int, uint, bool
// ============================================================================

float test_return_float_simple() {
    // Return float value
    float get_pi() {
        return 3.14159;
    }

    return get_pi();
}

// run: test_return_float_simple() ~= 3.14159

int test_return_int_simple() {
    // Return int value
    int get_answer() {
        return 42;
    }

    return get_answer();
}

// run: test_return_int_simple() == 42

uint test_return_uint_simple() {
    // Return uint value
    uint get_count() {
        return 100u;
    }

    return get_count();
}

// run: test_return_uint_simple() == 100u

bool test_return_bool_simple() {
    // Return bool value
    bool get_truth() {
        return true;
    }

    return get_truth();
}

// run: test_return_bool_simple() == true

float test_return_float_calculation() {
    // Return result of calculation
    float calculate_area(float radius) {
        return 3.14159 * radius * radius;
    }

    return calculate_area(2.0);
}

// run: test_return_float_calculation() ~= 12.56636

int test_return_int_arithmetic() {
    // Return result of integer arithmetic
    int add_numbers(int a, int b, int c) {
        return a + b + c;
    }

    return add_numbers(1, 2, 3);
}

// run: test_return_int_arithmetic() == 6

bool test_return_bool_logic() {
    // Return result of boolean logic
    bool is_even(int x) {
        return (x % 2) == 0;
    }

    return is_even(4) && !is_even(3);
}

// run: test_return_bool_logic() == true

float test_return_float_conversion() {
    // Return float converted from int
    float int_to_float(int x) {
        return float(x);
    }

    return int_to_float(5);
}

// run: test_return_float_conversion() ~= 5.0

int test_return_int_from_bool() {
    // Return int converted from bool
    int bool_to_int(bool b) {
        return b ? 1 : 0;
    }

    return bool_to_int(true) + bool_to_int(false);
}

// run: test_return_int_from_bool() == 1
