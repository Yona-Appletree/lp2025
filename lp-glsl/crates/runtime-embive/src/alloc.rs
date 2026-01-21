use core::ptr::addr_of;
use core::sync::atomic::{AtomicUsize, Ordering};

use linked_list_allocator::LockedHeap;

use crate::get_heap;

/// Track allocation statistics
static TOTAL_ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static TOTAL_DEALLOCATED: AtomicUsize = AtomicUsize::new(0);
static PEAK_USAGE: AtomicUsize = AtomicUsize::new(0);
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Get current memory usage
pub fn get_memory_usage() -> (usize, usize, usize, usize) {
    let allocated = TOTAL_ALLOCATED.load(Ordering::Relaxed);
    let deallocated = TOTAL_DEALLOCATED.load(Ordering::Relaxed);
    let current = allocated - deallocated;
    let peak = PEAK_USAGE.load(Ordering::Relaxed);
    let count = ALLOC_COUNT.load(Ordering::Relaxed);
    (current, peak, allocated, count)
}

/// Reset memory statistics
pub fn reset_memory_stats() {
    TOTAL_ALLOCATED.store(0, Ordering::Relaxed);
    TOTAL_DEALLOCATED.store(0, Ordering::Relaxed);
    PEAK_USAGE.store(0, Ordering::Relaxed);
    ALLOC_COUNT.store(0, Ordering::Relaxed);
}

/// Get the end of heap address from linker script
///
/// Returns the end address of the heap region (__heap_end).
fn get_heap_end() -> usize {
    unsafe extern "C" {
        static __heap_end: u8;
    }

    // __heap_end is a linker symbol pointing to the end of the heap section
    addr_of!(__heap_end) as usize
}

/// Base allocator instance
static BASE_ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Tracking allocator wrapper
struct TrackingAllocator;

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

unsafe impl core::alloc::GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let result = BASE_ALLOCATOR.lock().allocate_first_fit(layout);

        let ptr = match result {
            Ok(nn) => {
                let p = nn.as_ptr();

                // Track allocation
                let size = layout.size();
                let old_total = TOTAL_ALLOCATED.fetch_add(size, Ordering::Relaxed);
                let new_total = old_total + size;
                let deallocated = TOTAL_DEALLOCATED.load(Ordering::Relaxed);
                let current = new_total - deallocated;

                // Update peak
                let mut peak = PEAK_USAGE.load(Ordering::Relaxed);
                while current > peak {
                    match PEAK_USAGE.compare_exchange_weak(
                        peak,
                        current,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => break,
                        Err(x) => peak = x,
                    }
                }

                ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
                p
            }
            Err(e) => {
                // Allocation failed - print debug info before panicking
                unsafe extern "C" {
                    fn _print(ptr: *const u8, len: usize);
                }

                // We can't use format! here, so build message manually
                let msg = b"[ALLOC FAILED] size=";
                unsafe {
                    _print(msg.as_ptr(), msg.len());
                }

                // Print size as decimal (simple approach)
                let mut size_buf = [0u8; 32];
                let mut size_str_len = 0;
                let mut n = layout.size();
                if n == 0 {
                    size_buf[0] = b'0';
                    size_str_len = 1;
                } else {
                    let mut temp = n;
                    let mut digits = 0;
                    while temp > 0 {
                        digits += 1;
                        temp /= 10;
                    }
                    for i in (0..digits).rev() {
                        size_buf[i] = b'0' + (n % 10) as u8;
                        n /= 10;
                    }
                    size_str_len = digits;
                }
                unsafe {
                    _print(size_buf.as_ptr(), size_str_len);
                }

                let msg2 = b" align=";
                unsafe {
                    _print(msg2.as_ptr(), msg2.len());
                }

                let mut align_buf = [0u8; 32];
                let mut align_str_len = 0;
                let mut n = layout.align();
                if n == 0 {
                    align_buf[0] = b'0';
                    align_str_len = 1;
                } else {
                    let mut temp = n;
                    let mut digits = 0;
                    while temp > 0 {
                        digits += 1;
                        temp /= 10;
                    }
                    for i in (0..digits).rev() {
                        align_buf[i] = b'0' + (n % 10) as u8;
                        n /= 10;
                    }
                    align_str_len = digits;
                }
                unsafe {
                    _print(align_buf.as_ptr(), align_str_len);
                }

                let msg3 = b"\n";
                unsafe {
                    _print(msg3.as_ptr(), msg3.len());
                }

                core::ptr::null_mut()
            }
        };

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        if let Some(nn) = core::ptr::NonNull::new(ptr) {
            unsafe {
                BASE_ALLOCATOR.lock().deallocate(nn, layout);
            }
            TOTAL_DEALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        }
    }
}

/// Initialize the global allocator
///
/// This must be called before any heap allocations are made.
/// It sets up the allocator to use the memory region from `_end` to a reasonable heap end.
/// We limit the heap to 1MB to avoid issues with VM memory constraints.
pub unsafe fn init_allocator() {
    let heap_start = get_heap();

    // Ensure heap_start is in RAM region (>= RAM_OFFSET = 0x80000000)
    // If it's not, something is wrong with the linker script
    const RAM_OFFSET: usize = 0x80000000;
    if heap_start < RAM_OFFSET {
        // Heap start is not in RAM, skip initialization
        return;
    }

    // Calculate heap size from linker script symbols
    // Use __heap_end from the linker script, which marks the end of the heap section
    let heap_end_addr = get_heap_end();
    let heap_size = if heap_end_addr > heap_start {
        heap_end_addr - heap_start
    } else {
        // Fallback: use a conservative 512KB if symbols are invalid
        512 * 1024
    };

    // Ensure we have a valid heap size
    if heap_size == 0 {
        // No heap available, allocator will fail gracefully
        return;
    }

    // Initialize the allocator with the heap region
    // The allocator will manage memory starting at heap_start
    // Ensure heap_start is properly aligned (16 bytes for linked_list_allocator)
    // linked_list_allocator requires 16-byte alignment
    let aligned_heap_start = (heap_start + 15) & !15;

    // Ensure the heap start is at or after RAM_OFFSET
    let safe_heap_start = aligned_heap_start.max(RAM_OFFSET);

    // Recalculate heap size based on the safe start
    let safe_heap_size = if safe_heap_start > heap_start {
        heap_size - (safe_heap_start - heap_start)
    } else {
        heap_size
    };

    if safe_heap_size < 256 {
        // Not enough space after safety margin
        return;
    }

    // Initialize the base allocator
    // The allocator will write to memory starting at safe_heap_start
    // NOTE: safe_heap_start is a guest virtual address. When the allocator
    // tries to write to memory, those writes go through the interpreter's
    // memory abstraction (SliceMemory), which translates guest addresses to
    // offsets in the VM's RAM buffer.
    unsafe {
        BASE_ALLOCATOR
            .lock()
            .init(safe_heap_start as *mut u8, safe_heap_size);
    }
}
