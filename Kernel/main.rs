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
#![no_std]	//< Kernels can't use std
#![crate_name="kernel"]

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

// Kernel entrypoint (called by arch/<foo>/start.S)
#[no_mangle]
pub fn kmain()
{
	log!("Graddadwy Research Kernel version {}.{}.{}-git", 0, 0, 1);

    /* Initialize the early architecture. */
    let (mem_sz, page_sz): (u32, u32) = arch::early_init();

    log!("Available memory: {} bytes (~{} MB)", mem_sz, mem_sz / 1024 / 1024);
    log!("Default page size: {} bytes", page_sz);

    /* Initialize the physical memory manager. */
    let mut fma = mm::pmm::init(mem_sz, page_sz);

    /* Remap the kernel, so we take control of the paging structures. */
    let mut vmm = mm::vmm::remap_kernel();

    log!("Looping...");
	loop {}
}
