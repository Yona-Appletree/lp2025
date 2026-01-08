//! Separate file to test assembly generation
//! Compile with: rustc --emit asm asm_test.rs

#[no_mangle]
pub extern "C" fn test_structreturn_apple_aarch64(buffer: *mut f32) {
    unsafe {
        *buffer.add(0) = 1.0;
        *buffer.add(1) = 2.0;
        *buffer.add(2) = 3.0;
    }
}

#[no_mangle]
pub extern "C" fn test_structreturn_systemv(buffer: *mut f32) {
    unsafe {
        *buffer.add(0) = 1.0;
        *buffer.add(1) = 2.0;
        *buffer.add(2) = 3.0;
    }
}

