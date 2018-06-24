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
pub const KERNEL_BASE: usize = 0xFFFFFFFF80000000;
pub const PAGE_SHIFT: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;

extern crate multiboot;

use self::multiboot::{Multiboot, PAddr};
use core::slice;
use core::mem;

extern crate x86_mp;
use x86_mp::{MPEntryCode, MPFloatingPointer, MPConfigurationTableHeader};

use mm::vmm::Pml4Entry;
use mm::pmm::FrameAllocator;

pub fn paddr_to_slice<'a>(p: multiboot::PAddr, sz: usize) -> Option<&'a [u8]> {
    unsafe {
        let ptr = mem::transmute(p as usize + KERNEL_BASE);
        Some(slice::from_raw_parts(ptr, sz))
    }
}

/*
 * This method is responsible for discovering the available memory
 * on the system.
 */
unsafe fn discover_memory() -> usize
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
    return (mb.upper_memory_bound().unwrap() * 1024) as usize;
}

unsafe fn find_mp_tables() -> usize
{
    let base_mem_location: *const u16 = (KERNEL_BASE + 0x413) as *const u16;
    let base_mem_size: u16 = *base_mem_location;
    let base_mem_end: usize = (base_mem_size as usize) << 10;
    let search_mem_start: usize = (base_mem_end - (2 << 10));
    log!("Base memory size: {} KiB => [0x0 - 0x{:x}]", base_mem_size, base_mem_end);

    let mut search_now: *const u32 = (KERNEL_BASE + search_mem_start) as *const u32;
    loop {
        if (search_now as usize) >= base_mem_end {
            log!("Didn't find MP tables in base memory");
            break;
        }

        if *search_now == 0x5F504D5F {
            log!("found MP tables at 0x{:016x}", search_now as usize);
            return 0;
        }

        search_now = ((search_now as usize) + 16) as *const u32;
    }

    search_now = (KERNEL_BASE + 0x9fc00) as *const u32;
    loop {
        if (search_now as usize) >= (KERNEL_BASE + 0x9ffff) {
            log!("Didn't find MP tables in EBDA memory");
            break;
        }

        if *search_now == 0x5F504D5F {
            log!("found MP tables at 0x{:016x} in EBDA", search_now as usize);
            return 0;
        }

        search_now = ((search_now as usize) + 16) as *const u32;
    }

    search_now = (KERNEL_BASE + 0xa000) as *const u32;
    loop {
        if (search_now as usize) >= (KERNEL_BASE + 0xfffff) {
            log!("Didn't find MP tables in ROM memory");
            break;
        }

        if *search_now == 0x5F504D5F {
            log!("found MP tables at 0x{:016x} in ROM", search_now as usize);
            return search_now as usize;
        }

        search_now = ((search_now as usize) + 16) as *const u32;
    }
    0
}

pub unsafe fn set_page_directory(pml4: usize)
{
    log!("page directory is {}", pml4);
    asm!("mov $0, %cr3" :: "r" (pml4) : "memory")
}

pub unsafe fn get_page_directory<'a>() -> &'a mut [Pml4Entry]
{
    let mut value: u64;
    asm!("mov %cr3, $0" :"=r" (value) :: "memory");
    value = value + ::arch::KERNEL_BASE as u64;
    slice::from_raw_parts_mut(mem::transmute(value), 512)
}

pub fn early_init() -> (usize, usize)
{
    log!("Initializing AMD64 processors");
    let available_memory: usize;
    let mp_table_location: *const MPFloatingPointer;

    /* Discover the available memory. */
    unsafe {
        available_memory = discover_memory();
    }

    (available_memory, PAGE_SIZE)
}

fn enumerate_processors() -> usize
{
    let mp_ptr_location = unsafe { find_mp_tables() as *const MPFloatingPointer };
    let mp_ptr: MPFloatingPointer;

    if mp_ptr_location as usize != 0 {
        unsafe {
            mp_ptr = *mp_ptr_location;
        }
    } else {
        log!("MP Table not found, assuming 1 CPU");
        return 1;
    }

    if (mp_ptr.is_valid()) {
        log!("Enumerating available processors...");
        let mut processors = 0;
        unsafe {
            let mp_hdr_loc = KERNEL_BASE + mp_ptr.physical_address_pointer as usize;
            let mp_hdr: MPConfigurationTableHeader = *(mp_hdr_loc as *const MPConfigurationTableHeader);
            log!("MP header has {} entries, at 0x{:016x}, LAPIC at 0x{:016x}",
                 mp_hdr.entry_count, mp_hdr_loc, mp_hdr.local_apic_addr);

            let mp_hdr_iter = mp_hdr.iter(mp_hdr_loc);
            for i in mp_hdr_iter {
                if i == MPEntryCode::Processor {
                    processors += 1;
                }
            }
        };
        log!("Found {} processors in total", processors);
        return processors;
    } else {
        log!("MP table was invalid, assuming 1 CPU");
        return 1;
    }
}

pub fn late_init(fma: &mut FrameAllocator)
{
    enumerate_processors();
}
