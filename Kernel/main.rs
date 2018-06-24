/*
 * Rust BareBones OS
 * - By John Hodge (Mutabah/thePowersGang) 
 *
 * main.rs
 * - Top-level file for kernel
 *
 * This code has been put into the public domain, there are no restrictions on
 * its use, and the author takes no liability.
 */
#![feature(panic_implementation,panic_info_message)]	//< Panic handling
#![feature(asm)]	//< As a kernel, we need inline assembly
#![feature(alloc)]	// Allocation stuff
#![feature(allocator_api)]	// Messing with the allocator API
#![feature(const_fn)]	// Constant functions with CPFE
#![feature(language_items)]	// Language items!
#![feature(lang_items)]	// Language items!
#![no_std]	//< Kernels can't use std
#![crate_name="kernel"]

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate bitflags;

extern crate rlibc;
extern crate x86_mp;

// Macros, need to be loaded before everything else due to how rust parses.
#[macro_use]
mod macros;

// Achitecture-specific modules.
#[cfg(target_arch="x86_64")] #[path="arch/amd64/mod.rs"]
pub mod arch;
#[cfg(target_arch="x86")] #[path="arch/x86/mod.rs"]
pub mod arch;

// Exception handling (panic).
pub mod unwind;

// Logging code.
mod logging;

// Memory management.
mod mm;
use mm::alloc::{SimpleBumpAllocator, HEAP_START, HEAP_SIZE};
#[global_allocator]
static HEAP_ALLOCATOR: SimpleBumpAllocator
    = SimpleBumpAllocator::new(HEAP_START, HEAP_START + HEAP_SIZE);

use alloc::boxed::Box;
use alloc::Vec;

// Kernel entrypoint (called by arch/<foo>/start.S)
#[no_mangle]
pub fn kmain()
{
	log!("Graddadwy Research Kernel version {}.{}.{}-git", 0, 0, 1);

    /* Initialize the early architecture. */
    let (mem_sz, page_sz): (usize, usize) = arch::early_init();

    log!("Available memory: {} bytes (~{} MB)", mem_sz, mem_sz / 1024 / 1024);
    log!("Default page size: {} bytes", page_sz);

    /* Initialize the physical memory manager. */
    let mut fma = mm::pmm::init(mem_sz as u32, page_sz as u32);

    /* Remap the kernel, so we take control of the paging structures. */
    mm::vmm::remap_kernel(&mut fma);

    /* Initialize the allocator, so that we can use Boxed types */
    let test_fr = fma.allocate_frame();
    mm::vmm::map_addr_current(&mut fma, test_fr.frame_addr(), HEAP_START);

    //let box_test = Box::new(42);
    //let mut vec_test: Vec<usize> = vec![1, 2, 3, 4];
    //log!("{:?} {:?}", box_test, vec_test);
    //vec_test.push(5);
    //log!("{:?} {:?}", box_test, vec_test);

    arch::late_init(&mut fma);

    log!("Looping...");
	loop {}
}
