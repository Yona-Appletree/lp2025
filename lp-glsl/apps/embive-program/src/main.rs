#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate runtime_embive;

use core::panic::PanicInfo;

use runtime_embive::{ebreak, panic_syscall, syscall};

mod toy_demo;

/// Panics will report to the host and then exit
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Try to get panic message
    use core::fmt::Write;

    // Create a buffer for the panic message
    let mut panic_msg_buf = [0u8; 256];
    let mut cursor = 0;

    // Format the panic message into our buffer
    struct BufWriter<'a> {
        buf: &'a mut [u8],
        cursor: &'a mut usize,
    }

    impl<'a> Write for BufWriter<'a> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let bytes = s.as_bytes();
            let remaining = self.buf.len() - *self.cursor;
            let to_write = bytes.len().min(remaining);
            if to_write > 0 {
                self.buf[*self.cursor..*self.cursor + to_write].copy_from_slice(&bytes[..to_write]);
                *self.cursor += to_write;
            }
            Ok(())
        }
    }

    let mut writer = BufWriter {
        buf: &mut panic_msg_buf,
        cursor: &mut cursor,
    };

    // Try to format the full panic info
    let _ = write!(writer, "{}", info.message());

    // If message is empty, try payload
    if cursor == 0 {
        if let Some(payload) = info.payload().downcast_ref::<&str>() {
            let bytes = payload.as_bytes();
            let to_copy = bytes.len().min(panic_msg_buf.len());
            panic_msg_buf[..to_copy].copy_from_slice(&bytes[..to_copy]);
            cursor = to_copy;
        } else {
            let default_msg = b"panic occurred (no message)";
            let to_copy = default_msg.len().min(panic_msg_buf.len());
            panic_msg_buf[..to_copy].copy_from_slice(&default_msg[..to_copy]);
            cursor = to_copy;
        }
    }

    // Try to extract location info
    let (file_ptr, file_len, line) = if let Some(loc) = info.location() {
        let file = loc.file().as_bytes();
        (file.as_ptr(), file.len(), loc.line())
    } else {
        (core::ptr::null(), 0, 0)
    };

    // Report panic to host with the message
    panic_syscall(panic_msg_buf.as_ptr(), cursor, file_ptr, file_len, line);
}

/// Interrupt handler
/// This function is called when an interruption occurs
#[unsafe(no_mangle)]
fn interrupt_handler(_value: i32) {
    // Handle interrupts if needed
}

/// Main program - Phase 1: Hello World (JIT demo disabled - too heavy for emulator)
#[unsafe(no_mangle)]
pub extern "Rust" fn main() {
    // Phase 1: Hello World
    println!("Hello from RISC-V!");
    println!("Running in no_std with Cranelift-compiled code.");
    println!("This is a test of the toolchain.");
    println!("Successfully executed basic no_std program!");
    println!("");

    // Phase 2: Toy Language Runtime Compilation with Memory Tracking!
    println!("Testing Toy Language compilation with memory tracking...");

    let result = toy_demo::run_toy_demo();

    // Report result
    println!("");
    println!("Test result: {}", result);

    // Also report via syscall for test verification
    let args = [result, 0, 0, 0, 0, 0, 0];
    let _ = syscall(0, &args);

    // Exit successfully
    ebreak()
}
