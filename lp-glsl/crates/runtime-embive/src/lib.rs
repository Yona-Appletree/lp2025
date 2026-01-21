#![no_std]
#![allow(dead_code)]

use core::{
    arch::{asm, global_asm},
    mem::zeroed,
    num::NonZeroI32,
    option::Option::{None, Some},
    ptr::{addr_of, addr_of_mut, read, write_volatile},
    result::Result,
};

use critical_section::{Impl, RawRestoreState, set_impl};

mod alloc;
mod print;

pub use alloc::{get_memory_usage, init_allocator, reset_memory_stats};

pub use print::_print;

/// Number of syscall arguments
pub const SYSCALL_ARGS: usize = 7;

/// Syscall numbers
pub const SYSCALL_DONE: i32 = 0;
pub const SYSCALL_PANIC: i32 = 1;
pub const SYSCALL_WRITE: i32 = 2;

// Critical section implementation
struct EmbiveCriticalSection;
set_impl!(EmbiveCriticalSection);

unsafe impl Impl for EmbiveCriticalSection {
    unsafe fn acquire() -> RawRestoreState {
        disable_interrupts()
    }

    unsafe fn release(previous: RawRestoreState) {
        if previous {
            enable_interrupts();
        }
    }
}

/// System Call. Must be implemented by the host.
///
/// Parameters:
/// - nr: System call number
/// - args: Array of arguments
///
/// Returns:
/// - Ok(value): The system call was successful.
/// - Err(error): The system call failed.
pub fn syscall(nr: i32, args: &[i32; SYSCALL_ARGS]) -> Result<i32, NonZeroI32> {
    let error: i32;
    let value: i32;

    unsafe {
        asm!(
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

    match NonZeroI32::new(error) {
        Some(error) => Result::<i32, NonZeroI32>::Err(error),
        None => Result::<i32, NonZeroI32>::Ok(value),
    }
}

/// Wait For Interrupt
///
/// Ask the host to put the interpreter to sleep until an interruption occurs
/// May return without any interruption.
#[inline(always)]
pub fn wfi() {
    unsafe {
        asm!("wfi", options(nostack));
    }
}

/// Report a panic to the host VM
///
/// This should be called from the panic handler before ebreak.
/// args[0] = panic message pointer (as i32)
/// args[1] = panic message length
/// args[2] = file pointer (as i32, 0 if unavailable)
/// args[3] = file length
/// args[4] = line number (0 if unavailable)
pub fn panic_syscall(
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
pub fn ebreak() -> ! {
    unsafe {
        asm!("ebreak", options(nostack, noreturn));
    }
}

/// Enable Interrupts
///
/// Set the `mstatus.MIE` bit to 1
///
/// Returns the previous state of the `mstatus.MIE` bit
#[inline(always)]
pub fn enable_interrupts() -> bool {
    let mut mstatus: usize;
    unsafe {
        asm!("csrrsi {}, mstatus, 8", out(reg) mstatus);
    }

    (mstatus & 8) != 0
}

/// Disable Interrupts
///
/// Set the `mstatus.MIE` bit to 0
///
/// Returns the previous state of the `mstatus.MIE` bit
#[inline(always)]
pub fn disable_interrupts() -> bool {
    let mut mstatus: usize;
    unsafe {
        asm!("csrrci {}, mstatus, 8", out(reg) mstatus);
    }

    (mstatus & 8) != 0
}

/// Get heap address from linker script
///
/// Returns the heap start address (memory address after data and stack).
/// Any leftover memory allocated by the host can be used as heap.
pub fn get_heap() -> usize {
    unsafe extern "C" {
        static _end: u8;
    }

    addr_of!(_end) as usize
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

// Interrupt trap (removed - not needed for emulator testing)

/// This code is responsible for initializing the .bss and .data sections, and calling the user's main function.
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
            write_volatile(sdata, read(sdatas));
            sdata = sdata.offset(1);
            sdatas = sdatas.offset(1);
        }
    }

    // Initialize allocator before calling main
    unsafe {
        init_allocator();
    }

    // Call user's main function (must be provided by the program crate)
    unsafe extern "Rust" {
        fn main();
    }

    unsafe {
        main();
    }

    // Exit the interpreter
    ebreak()
}
