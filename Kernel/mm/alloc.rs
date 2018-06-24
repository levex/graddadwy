#[path="../logging.rs"]
mod logging;

use core::sync::atomic::{AtomicUsize, Ordering};
use core::ptr::NonNull;
use alloc::alloc::{Alloc, AllocErr, Layout, GlobalAlloc};

pub const HEAP_START: usize = 2 * 1024 * 1024 + ::arch::KERNEL_BASE;
pub const HEAP_SIZE: usize = 128 * 1024; /* 128KiB */

#[derive(Debug)]
pub struct SimpleBumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: AtomicUsize,
}

impl SimpleBumpAllocator {
    pub const fn new(heap_start: usize, heap_end: usize) -> Self {
        Self { heap_start, heap_end, next: AtomicUsize::new(heap_start), }
    }
}

#[lang = "oom"]
#[no_mangle]
pub fn rust_oom() -> ! {
    panic!("Out of memory!");
}

pub fn align_down(addr: usize, align: usize) -> usize {
    if align.is_power_of_two() {
        addr & !(align - 1)
    } else if align ==  0 {
        align
    } else {
        panic!("align_down called with align {:?}, that is not a power of two");
    }
}

pub fn align_up(addr: usize, align: usize) -> usize {
    align_down(addr + align - 1, align)
}

unsafe impl GlobalAlloc for SimpleBumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        loop {
            let curr_next = self.next.load(Ordering::Relaxed);
            let alloc_start = align_up(curr_next, layout.align());
            let alloc_end = alloc_start.saturating_add(layout.size());

            if alloc_end <= self.heap_end {
                let next_now = self.next.compare_and_swap(curr_next, alloc_end,
                                                          Ordering::Relaxed);
                if next_now == curr_next {
                    log!("allocated {} bytes", alloc_end - alloc_start);
                    return alloc_start as *mut u8;
                }
            } else {
                return 0 as *mut u8;
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        log!("WARNING: leaking memory: {:?}", layout);
    }
}
