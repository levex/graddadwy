/*
 * Rust BareBones OS
 * - By John Hodge (Mutabah/thePowersGang) 
 *
 * arch/amd64/mod.rs
 * - Top-level file for amd64 architecture
 *
 * == LICENCE ==
 * This code has been put into the public domain, there are no restrictions on
 * its use, and the author takes no liability.
 */

// x86 port IO 
#[path = "../x86_common/io.rs"]
mod x86_io;

// Debug output channel (uses serial)
#[path = "../x86_common/debug.rs"]
pub mod debug;

// Logging code
#[path = "../../logging.rs"]
mod logging;

/* Some globals. */
pub const KERNEL_BASE: u64 = 0xFFFFFFFF80000000;
pub const PAGE_SHIFT: u32 = 12;
pub const PAGE_SIZE: u32 = 1 << PAGE_SHIFT;

extern crate multiboot;

use self::multiboot::{Multiboot, PAddr};
use core::slice;
use core::mem;

pub fn paddr_to_slice<'a>(p: multiboot::PAddr, sz: usize) -> Option<&'a [u8]> {
    unsafe {
        let ptr = mem::transmute(p as u64 + KERNEL_BASE);
        Some(slice::from_raw_parts(ptr, sz))
    }
}

/*
 * This method is responsible for discovering the available memory
 * on the system.
 */
unsafe fn discover_memory() -> u32
{
    /* External variables, where we saved the multiboot data.
     * These are defined in arch/amd64/start.S
     */
    extern {
        static mboot_sig: u32;
        static mboot_ptr: u32;
    }

    if mboot_sig != 0x2badb002 {
        panic!("Invalid Multiboot signature: 0x{:x}", mboot_sig);
    }

    /* First, try the Multiboot-provided memory map. */
    let mb = Multiboot::new(mboot_ptr as multiboot::PAddr, paddr_to_slice).unwrap();

    /* Print the memory map. */
    for area in mb.memory_regions().unwrap() {
        log!("[0x{:x} - 0x{:x}] (length: {} Kb): {:?}",
            area.base_address(), area.base_address() + area.length(),
            area.length() / 1024, area.memory_type());
    }

    /* Calculate the available memory that's in "high memory". */
    return mb.upper_memory_bound().unwrap() * 1024;
}

pub fn early_init() -> (u32, u32)
{
    log!("Initializing AMD64 processors");
    let available_memory: u32;

    /* Discover the available memory. */
    unsafe {
        available_memory = discover_memory();
    }

    (available_memory, PAGE_SIZE)
}
