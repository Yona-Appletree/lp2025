#![no_std]
#![no_main]

mod builtin_refs;
mod host;
mod print;

// Re-export _print so macros can find it
pub use print::_print;

use core::arch::asm;
use core::{
    arch::global_asm,
    fmt::Write,
    mem::zeroed,
    ptr::{addr_of_mut, read, write_volatile},
};
use lp_builtins::host_debug;

/// Syscall number for panic
const SYSCALL_PANIC: i32 = 1;

/// Number of syscall arguments
const SYSCALL_ARGS: usize = 7;

/// System call implementation
fn syscall(nr: i32, args: &[i32; SYSCALL_ARGS]) -> i32 {
    let error: i32;
    let value: i32;
    unsafe {
        core::arch::asm!(
            "ecall",
            in("x17") nr,
            inlateout("x10") args[0] => error,
            inlateout("x11") args[1] => value,
            in("x12") args[2],
            in("x13") args[3],
            in("x14") args[4],
            in("x15") args[5],
            in("x16") args[6],
        );
    }
    if error != 0 { error } else { value }
}

/// Report a panic to the host VM
///
/// This should be called from the panic handler before ebreak.
/// args[0] = panic message pointer (as i32)
/// args[1] = panic message length
/// args[2] = file pointer (as i32, 0 if unavailable)
/// args[3] = file length
/// args[4] = line number (0 if unavailable)
fn panic_syscall(
    msg_ptr: *const u8,
    msg_len: usize,
    file_ptr: *const u8,
    file_len: usize,
    line: u32,
) -> ! {
    let args = [
        msg_ptr as i32,
        msg_len as i32,
        file_ptr as i32,
        file_len as i32,
        line as i32,
        0,
        0,
    ];
    let _ = syscall(SYSCALL_PANIC, &args);
    ebreak()
}

/// Exit the interpreter
#[inline(always)]
fn ebreak() -> ! {
    unsafe {
        asm!("ebreak", options(nostack, noreturn))
    }
}

/// Panic handler
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
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

    // If message is empty, use default message
    if cursor == 0 {
        let default_msg = b"panic occurred (no message)";
        let to_copy = default_msg.len().min(panic_msg_buf.len());
        panic_msg_buf[..to_copy].copy_from_slice(&default_msg[..to_copy]);
        cursor = to_copy;
    }

    // Try to extract location info
    // Note: In no_std, location() may return None if location tracking is disabled
    // or if the panic was created without location info
    // The file name from Location is a string literal in the binary, so the pointer is valid
    let (file_ptr, file_len, line) = if let Some(loc) = info.location() {
        let file = loc.file();
        // file() returns &str which points to a string literal in the binary
        // This pointer is valid for the lifetime of the program
        let file_bytes = file.as_bytes();
        (file_bytes.as_ptr(), file_bytes.len(), loc.line())
    } else {
        (core::ptr::null(), 0, 0)
    };

    // Report panic to host with the message
    panic_syscall(panic_msg_buf.as_ptr(), cursor, file_ptr, file_len, line);
}

// Binary entry point
// Initializes the global, stack, and frame pointers; and then calls the _code_entry function
global_asm! {
    ".section .text.init.entry, \"ax\"",
    ".global _entry",
    "_entry:",
    ".option push",
    ".option norelax",
    ".option norvc",  // Disable compressed instructions
    // Initialize global pointer
    "la gp, __global_pointer$",
    // Initialize stack and frame pointers
    "la t1, __stack_start",
    "andi sp, t1, -16",
    "add s0, sp, zero",
    ".option pop",
    // Call _code_entry using long-range jump (la pseudo-instruction expands to auipc + addi)
    "la t0, _code_entry",
    "jalr ra, 0(t0)",
}

/// This code is responsible for initializing the .bss and .data sections, and calling the placeholder main function.
/// The main function will then optionally call user _init if present.
#[unsafe(no_mangle)]
unsafe extern "C" fn _code_entry() -> ! {
    unsafe extern "C" {
        // These symbols come from `memory.ld`
        static mut __bss_target_start: u32; // Start of .bss target
        static mut __bss_target_end: u32; // End of .bss target
        static mut __data_target_start: u32; // Start of .data target
        static mut __data_target_end: u32; // End of .data target
        static __data_source_start: u32; // Start of .data source
    }

    // Initialize (Zero) BSS
    let mut sbss: *mut u32 = addr_of_mut!(__bss_target_start);
    let ebss: *mut u32 = addr_of_mut!(__bss_target_end);

    while sbss < ebss {
        unsafe {
            write_volatile(sbss, zeroed());
            sbss = sbss.offset(1);
        }
    }

    // Initialize Data
    let mut sdata: *mut u32 = addr_of_mut!(__data_target_start);
    let edata: *mut u32 = addr_of_mut!(__data_target_end);
    let mut sdatas: *const u32 = unsafe { &__data_source_start };

    while sdata < edata {
        unsafe {
            let val = read(sdatas);
            write_volatile(sdata, val);
            sdata = sdata.offset(1);
            sdatas = sdatas.offset(1);
        }
    }

    // Call placeholder main function
    unsafe extern "C" {
        fn main();
    }

    unsafe {
        main();
    }

    unsafe {
        asm! { "ebreak" }
    }

    loop {}
}

/// User _init pointer - will be overwritten by object loader to point to actual user _init()
/// Initialized to sentinel value 0xDEADBEEF to make it obvious if relocation isn't applied
#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".data")]
static mut __USER_MAIN_PTR: u32 = 0xDEADBEEF;

/// Placeholder main function that references all builtin functions to prevent dead code elimination.
///
/// This function:
/// 1. References all __lp_* functions explicitly (prevents dead code elimination)
/// 2. Reads __USER_MAIN_PTR from .data section
/// 3. Jumps to user _init if set, otherwise halts gracefully
#[unsafe(no_mangle)]
pub extern "C" fn main() -> () {
    // Reference all builtin functions to prevent dead code elimination
    // This is done via the generated builtin_refs module
    builtin_refs::ensure_builtins_referenced();

    // Reference host functions to prevent dead code elimination
    unsafe {
        let _debug_fn: extern "C" fn(*const u8, usize) = host::__host_debug;
        let _println_fn: extern "C" fn(*const u8, usize) = host::__host_println;
        let _ = core::ptr::read_volatile(&_debug_fn as *const _);
        let _ = core::ptr::read_volatile(&_println_fn as *const _);
    }

    // Read user _init pointer
    let user_init_ptr =
        unsafe { core::ptr::read_volatile(&raw const __USER_MAIN_PTR as *const u32) };

    if user_init_ptr == 0 || user_init_ptr == 0xDEADBEEF {
        // No user _init set - halt gracefully
        host_debug!("[lp-builtins-app::main()] no user _init specified. halting.");
        ebreak();
    }

    host_debug!(
        "[lp-builtins-app::main()] jumping to user _init at 0x{:x}",
        user_init_ptr
    );

    // Jump to user _init
    // On RISC-V 32-bit, function pointers are 32 bits, so we can safely cast u32 to fn pointer
    // We use a pointer cast to avoid transmute size mismatch on host compiler
    let res = unsafe {
        let user_init_ptr_usize = user_init_ptr as usize;
        let user_init: extern "C" fn() -> i32 = core::mem::transmute(user_init_ptr_usize);
        user_init()
    };

    host_debug!(
        "[lp-builtins-app::main()] returned from user _init(): {}",
        res
    );
}
