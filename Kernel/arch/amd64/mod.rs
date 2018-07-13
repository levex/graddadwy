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

#[path = "./apic.rs"]
mod apic;

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
use core::ptr;
use alloc::Vec;

extern crate x86_mp;
use x86_mp::{ProcessorEntry, MPEntryCode, MPFloatingPointer, MPConfigurationTableHeader};

use mm::vmm::Pml4Entry;
use mm::pmm::FrameAllocator;

pub fn paddr_to_slice<'a>(p: multiboot::PAddr, sz: usize) -> Option<&'a [u8]> {
    unsafe {
        let ptr = mem::transmute(p as usize + KERNEL_BASE);
        Some(slice::from_raw_parts(ptr, sz))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Processor {
    pub id: usize,
    pub apic_id: usize,
    pub stack_frame: usize,
}

pub static mut processor_list: Vec<Processor> = Vec::new();

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

    log!("Multiboot pointer: 0x{:016x}, Signature: 0x{:08x}", mboot_ptr, mboot_sig);

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

fn copy_smp_into_to(target_addr: usize, start: usize, end: usize)
{
    /* verify that the code there is correct */
    unsafe {
        let verify_ptr: *const u32 = (target_addr + ::arch::KERNEL_BASE) as *const u32;
        let data: u32 = *verify_ptr;
        log!("Data at 0x{:016x} is 0x{:08x}", target_addr, data);
    }
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

    extern {
        static SMP_AP_START: u32;
        static SMP_AP_END: u32;
    }
    let smp_ap_start_addr: usize = unsafe { mem::transmute(&SMP_AP_START) };
    let smp_ap_end_addr: usize = unsafe { mem::transmute(&SMP_AP_END) };
    log!("SMP_AP_START: [0x{:016x} - 0x{:016x}]",
         smp_ap_start_addr, smp_ap_end_addr);

    /* FIXME: this is not needed */
    copy_smp_into_to(0xA000, smp_ap_start_addr, smp_ap_end_addr);

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
    //log!("page directory is {}", pml4);
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

fn enumerate_processors(fma: &mut FrameAllocator) -> usize
{
    let mp_ptr_location = unsafe { find_mp_tables() as *const MPFloatingPointer };
    let mp_ptr: MPFloatingPointer;

    // extern {
        // static smp_ap_booted: u16;
    // }

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

            let lapic: apic::LAPIC = apic::LAPIC::new(fma, mp_hdr.local_apic_addr as usize, 0);

            let mp_hdr_iter = mp_hdr.iter(mp_hdr_loc);

            /* Count the number of processors */
            processors = mp_hdr_iter.clone().map(|x|
                                if x.code == MPEntryCode::Processor {
                                    1
                                } else {
                                    0
                                }).sum();

            /* Allocate the stack frames for the APs */
            for i in 0..processors {
                processor_list.push(Processor {
                    stack_frame: fma.allocate_frame().frame_addr(),
                    id: i,
                    apic_id: 0xffffffffffffffff,
                });
            }

            let mut p_c = 0;
            let mut b_c = 0;
            let mut __did_an_ap_boot: u32;
            let mut res: u32 = 0;

            /* Wake up the APs */
            for i in mp_hdr_iter {
                if i.code == MPEntryCode::Processor {
                    p_c = p_c + 1;
                }
                asm!("mfence; movl unique_stack_id, $0; movl did_an_ap_boot, $1":"=r"(res),"=r"(__did_an_ap_boot));
                log!("p_c = {} b_c = {} res = {} daab = {}", p_c, b_c, res, __did_an_ap_boot);
                //if p_c != 0 && p_c % 4 == 0 && i.code == MPEntryCode::Processor {
                if i.code == MPEntryCode::Processor && __did_an_ap_boot != b_c {
                    log!("Reached maximum parallel boot, waiting...");
                    loop {
                        asm!("mfence; movl unique_stack_id, $0; movl did_an_ap_boot, $1":"=r"(res),"=r"(__did_an_ap_boot));
                        //log!("Parallel boot hang: usi: {} daab: {}", res, __did_an_ap_boot);
                        if __did_an_ap_boot == b_c /* && res == 0 */ {
                            //asm!("movl $$0x0, did_an_ap_boot");
                            log!("Parallel boot hang done.");
                            break;
                        }
                    }
                }
                if i.code == MPEntryCode::Processor {
                    let proc = i.get_processor_entry().unwrap();
                    if proc.lapic_id == 0 {
                        continue;
                    }
                    lapic.send_init_to(proc.lapic_id);
                    let mut wait = 400000;
                    loop {
                        wait = wait - 1;
                        if wait == 0 {
                            break;
                        }
                    }
                    lapic.send_sipi_to(proc.lapic_id, 0xA);
                    let mut wait = 400000;
                    loop {
                        wait = wait - 1;
                        if wait == 0 {
                            break;
                        }
                    }
                    b_c += 1;
                }
            }
        };
        log!("Found {} processors in total", processors);
        // unsafe {
            // let smp_ap_booted_addr: usize = mem::transmute(&smp_ap_booted);
            // let smp_ap_booted_ptr: *const u16 = (KERNEL_BASE + smp_ap_booted_addr) as *const u16;
            // log!("{} processors booted", ptr::read_volatile(smp_ap_booted_ptr));
        // }
        return processors;
    } else {
        log!("MP table was invalid, assuming 1 CPU");
        return 1;
    }
}

pub unsafe fn new_cpu_init(cpu_id: usize)
{
    let _cpu_id: u32 = cpu_id as u32;

    /* save the current CPU id in the %fs register of the AP */
    asm!("xor %edx, %edx; movl $$0xC0000103, %ecx; wrmsr"
         ::"{rax}"(_cpu_id):"rdx","rcx");

    /* synchronize page tables */
    ::arch::set_page_directory(::mm::vmm::KERNEL_PAGE_DIRECTORY);

    /* get the processor structure for this AP */
    let this_ap = processor_list.iter().filter(|x| x.id == cpu_id).next().unwrap();

    /* allocate a new stack */
    let frame = this_ap.stack_frame + ::arch::PAGE_SIZE + ::arch::KERNEL_BASE;

    log!("AP {} about to switch stack to 0x{:016x}", cpu_id, frame);

    /* switch stacks */
    asm!("movq %rax, %rsp;
         movq %rax, %rbp;
         call new_cpu_init_tail"::"%rax"(frame));
}

#[no_mangle]
pub unsafe fn new_cpu_init_tail()
{
    let cpu_id: u32;

    //asm!("movl %fs, $0":"=r"(cpu_id));
    asm!("rdtscp":"={ecx}"(cpu_id)::"eax", "edx");

    /* get back the AP id */
    log!("Switched stack for AP {}", cpu_id);

    /* get the procesor structure */
    let this_ap = processor_list.iter().filter(|x|
                    x.id == cpu_id as usize
                    ).next().unwrap();

    /* TODO: allocate percpu storage */
    ::arch::allocate_percpu_storage<Processor>(cpu_id);

    /* TODO: set percpu storage in %gs */

    /* signal to the BSP that we are now done */
    asm!("lock decl unique_stack_id; lock incl did_an_ap_boot");

    loop {}
}

pub fn allocate_percpu_storage<T>(cpu_id: u32) -> PerCpu<T>
{
}

pub fn late_init(fma: &mut FrameAllocator)
{
    enumerate_processors(fma);
}
