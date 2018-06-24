#[path="../../logging.rs"]
mod logging;

use core::slice;
use core::mem;

use ::mm::pmm::{Frame, FrameAllocator};
use ::arch::PAGE_SHIFT;

pub type PhysAddr = usize;
pub type VirtAddr = usize;

/* Generic trait that describes all the tables that are used in Paging */
trait PagingTable {
    fn set_address(&mut self, addr: usize);
    fn get_address(&self) -> usize;
    fn clear(&mut self);
}

struct MyType<'a, T: 'a>(&'a mut [T]);

impl<'a, T: 'a> From<Frame> for MyType<'a, T>
    where T: PagingTable
{
    fn from(fr: Frame) -> MyType<'a, T>
    {
        unsafe {
            let ptr = mem::transmute(::arch::KERNEL_BASE + (fr.frame_id << PAGE_SHIFT));
            MyType(slice::from_raw_parts_mut(ptr, 512))
        }
    }
}

/* Structure describing the PML4E */
bitflags! {
    pub struct Pml4Entry : u64 {
        const PRESENT          = (1 << 0);
        const READWRITE        = (1 << 1);
        const USERSUPERVISOR   = (1 << 2);
        const PAGEWT           = (1 << 3);
        const PAGECACHEDISABLE = (1 << 4);
        const ACCESSED         = (1 << 5);
        const IGNORED          = (1 << 6);
        const MUSTBEZERO       = (3 << 7);
        const AVAILTOSOFTWARE  = (7 << 9);
    }
}
impl Pml4Entry {
    fn new_table<'a>(fr: Frame) -> &'a mut [Pml4Entry] {
        match MyType::from(fr) { MyType(a) => a }
    }
}
impl PagingTable for Pml4Entry {
    fn set_address(&mut self, addr: usize) {
        let clean_addr = addr as u64 & !((1 << ::arch::PAGE_SHIFT) - 1);
        self.bits = self.bits | clean_addr;
    }

    fn get_address(&self) -> usize {
        (self.bits as u64 & !((1 << ::arch::PAGE_SHIFT) - 1)) as usize
    }

    fn clear(&mut self) {
        self.bits = 0;
    }
}

/* Structure describing the PDPE */
bitflags! {
    struct PdpEntry : u64 {
        const PRESENT          = (1 << 0);
        const READWRITE        = (1 << 1);
        const USERSUPERVISOR   = (1 << 2);
        const PAGEWT           = (1 << 3);
        const PAGECACHEDISABLE = (1 << 4);
        const ACCESSED         = (1 << 5);
        const IGNORED          = (1 << 6);
        const MUSTBEZERO       = (3 << 7);
        const AVAILTOSOFTWARE  = (7 << 9);
    }
}
impl PdpEntry {
    fn new_table<'a>(fr: Frame) -> &'a mut [PdpEntry] {
        match MyType::from(fr) { MyType(a) => a }
    }
}
impl PagingTable for PdpEntry {
    fn set_address(&mut self, addr: usize) {
        let clean_addr = addr as u64 & !((1 << ::arch::PAGE_SHIFT) - 1);
        self.bits = self.bits | clean_addr;
    }

    fn get_address(&self) -> usize {
        (self.bits as u64 & !((1 << ::arch::PAGE_SHIFT) - 1)) as usize
    }

    fn clear(&mut self) {
        self.bits = 0;
    }
}

/* Structure describing the PDE */
bitflags! {
    struct PdEntry : u64 {
        const PRESENT          = (1 << 0);
        const READWRITE        = (1 << 1);
        const USERSUPERVISOR   = (1 << 2);
        const PAGEWT           = (1 << 3);
        const PAGECACHEDISABLE = (1 << 4);
        const ACCESSED         = (1 << 5);
        const IGNORED          = (1 << 6);
        const MUSTBEZERO       = (1 << 7);
        const IGNORED2         = (1 << 8);
        const AVAILTOSOFTWARE  = (7 << 9);
    }
}
impl PdEntry {
    fn new_table<'a>(fr: Frame) -> &'a mut [PdEntry] {
        match MyType::from(fr) { MyType(a) => a }
    }
}
impl PagingTable for PdEntry {
    fn set_address(&mut self, addr: usize) {
        let clean_addr = addr as u64 & !((1 << ::arch::PAGE_SHIFT) - 1);
        self.bits = self.bits | clean_addr;
    }

    fn get_address(&self) -> usize {
        (self.bits as u64 & !((1 << ::arch::PAGE_SHIFT) - 1)) as usize
    }

    fn clear(&mut self) {
        self.bits = 0;
    }
}

/* Structure describing the PTE */
bitflags! {
    struct PtEntry : u64 {
        const PRESENT          = (1 << 0);
        const READWRITE        = (1 << 1);
        const USERSUPERVISOR   = (1 << 2);
        const PAGEWT           = (1 << 3);
        const PAGECACHEDISABLE = (1 << 4);
        const ACCESSED         = (1 << 5);
        const DIRTY            = (1 << 6);
        const PAGEATTRTABLE    = (1 << 7);
        const GLOBAL           = (1 << 8);
        const AVAILTOSOFTWARE  = (7 << 9);
    }
}
impl PtEntry {
    fn new_table<'a>(fr: Frame) -> &'a mut [PtEntry] {
        match MyType::from(fr) { MyType(a) => a }
    }
}
impl PagingTable for PtEntry {
    fn set_address(&mut self, addr: usize) {
        let clean_addr = addr as u64 & !((1 << ::arch::PAGE_SHIFT) - 1);
        self.bits = self.bits | clean_addr;
    }

    fn get_address(&self) -> usize {
        (self.bits as u64 & !((1 << ::arch::PAGE_SHIFT) - 1)) as usize
    }

    fn clear(&mut self) {
        self.bits = 0;
    }
}

fn get_pml4_index_for(addr: usize) -> usize { (addr >> 39) & 0x1ff }
fn get_pdp_index_for(addr: usize) -> usize {  (addr >> 30) & 0x1ff } 
fn get_pd_index_for(addr: usize) -> usize {   (addr >> 21) & 0x1ff } 
fn get_pt_index_for(addr: usize) -> usize {   (addr >> 12) & 0x1ff } 

/* (PML4E, PDPE, PDE, PTE) */
pub fn get_address_indices_for(addr: usize) -> (usize, usize, usize, usize)
{
    (get_pml4_index_for(addr),
        get_pdp_index_for(addr),
        get_pd_index_for(addr),
        get_pt_index_for(addr)
        )
}

pub fn map_addr_in(pml4_table: &mut [Pml4Entry], fma: &mut FrameAllocator,
                  addr: usize, to: usize)
{
    let (pml4_idx, pdp_idx, pd_idx, pt_idx) = get_address_indices_for(to);
    let pdp_table: &mut [PdpEntry];
    let pd_table: &mut [PdEntry];
    let pt_table: &mut [PtEntry];

    if pml4_table[pml4_idx].contains(Pml4Entry::PRESENT) {
        unsafe {
            pdp_table = slice::from_raw_parts_mut(
                    mem::transmute(pml4_table[pml4_idx].get_address() + ::arch::KERNEL_BASE), 512);
        }
    } else {
        let pdp_fr = fma.allocate_frame();
        pdp_table = PdpEntry::new_table(pdp_fr);
        pml4_table[pml4_idx].clear();
        pml4_table[pml4_idx].set_address(pdp_fr.frame_addr());
        pml4_table[pml4_idx].set(Pml4Entry::PRESENT, true);
        pml4_table[pml4_idx].set(Pml4Entry::READWRITE, true);
    }

    if pdp_table[pdp_idx].contains(PdpEntry::PRESENT) {
        unsafe {
            pd_table = slice::from_raw_parts_mut(
                    mem::transmute(pdp_table[pdp_idx].get_address() + ::arch::KERNEL_BASE), 512);
        }
    } else {
        let pd_fr = fma.allocate_frame();
        pd_table = PdEntry::new_table(pd_fr);
        pdp_table[pdp_idx].clear();
        pdp_table[pdp_idx].set_address(pd_fr.frame_addr());
        pdp_table[pdp_idx].set(PdpEntry::PRESENT, true);
        pdp_table[pdp_idx].set(PdpEntry::READWRITE, true);
    }

    if pd_table[pd_idx].contains(PdEntry::PRESENT) {
        unsafe {
            pt_table = slice::from_raw_parts_mut(
                    mem::transmute(pd_table[pd_idx].get_address() + ::arch::KERNEL_BASE), 512);
        }
    } else {
        let pt_fr = fma.allocate_frame();
        pt_table = PtEntry::new_table(pt_fr);
        pd_table[pd_idx].clear();
        pd_table[pd_idx].set_address(pt_fr.frame_addr());
        pd_table[pd_idx].set(PdEntry::PRESENT, true);
        pd_table[pd_idx].set(PdEntry::READWRITE, true);
    }

    pt_table[pt_idx].clear();
    pt_table[pt_idx].set_address(addr);
    pt_table[pt_idx].set(PtEntry::PRESENT, true);
    pt_table[pt_idx].set(PtEntry::READWRITE, true);
}

pub fn map_addr_current(fma: &mut FrameAllocator, addr: usize, to: usize)
{
    unsafe {
        map_addr_in(::arch::get_page_directory(), fma, addr, to);
    }
}

pub fn remap_kernel<'a>(allocator: &mut FrameAllocator)
{
    let remap_target = ::arch::KERNEL_BASE as usize;

    log!("Attempting to remap the kernel to 0x{:x}, page 0x{:x}",
            remap_target, remap_target >> PAGE_SHIFT);

    /* TODO: This should be moved to architecture specific code. */
    /* Reserve a page for the PML4. */
    let pml4: Frame = allocator.allocate_frame();
    let pml4_table: &mut [Pml4Entry] = Pml4Entry::new_table(pml4);

    log!("Remap indices: (PML4E, PDPE, PDE, PTE) = {:?}",
        get_address_indices_for(remap_target));

    for i in 0..512 {
        let offset = i << ::arch::PAGE_SHIFT;
        /* TODO: these frames need to be marked as not free */
        map_addr_in(pml4_table, allocator, 0 + offset, remap_target + offset);
    }

    unsafe {
        ::arch::set_page_directory(pml4.frame_addr());
    }

    log!("Remap successful!");
}
