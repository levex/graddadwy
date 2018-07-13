#[path = "../../logging.rs"]
mod logging;

use core::mem;
use core::fmt;

#[derive(Clone, Copy)]
pub struct Frame {
    pub frame_id: usize,
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.frame_id)
    }
}

impl Frame {
    pub fn frame_addr_with(&self, page_size: usize) -> usize {
        self.frame_id
    }

    pub fn frame_addr(&self) -> usize {
        self.frame_id * (::arch::PAGE_SIZE)
    }

    fn get_frame_by_id(id: usize) -> Frame {
        Frame { frame_id: id }
    }

    fn get_frame_for(address: usize, page_size: usize) -> Frame {
        Frame::get_frame_by_id(address / page_size)
    }
}

pub struct FrameAllocator {
    next_free_frame: Frame,
    pub page_size: usize,
}

impl FrameAllocator {
    pub fn allocate_frame(&mut self) -> Frame {
        /* Return the next_free_frame, then increment to the next */
        let ret = self.next_free_frame.clone();
        self.next_free_frame =
            Frame::get_frame_by_id(ret.frame_id + 1);
        log!("Allocated frame id {} addr 0x{:x}", ret,
             ret.frame_addr());
        ret
    }

    pub fn free_frame(&mut self, frame: Frame) {
        /* FIXME: no-op */
    }
}

pub fn init(mem_size: u32, page_size: u32) -> FrameAllocator
{
    /* Determine the end of the kernel */
    extern "C" {
        /* Defined in the linker script */
        pub static kernel_end: u32;
    }
    let _kernel_end: usize = unsafe { mem::transmute(&kernel_end) };

    /* Check if we have enough RAM */
    if mem_size < 1 * 1024 * 1024 {
        panic!("This system has {} bytes of RAM, 
                which is insufficient for Graddadwy. (Need at least {} bytes free.)",
                    mem_size, 1 * 1024 * 1024);
    }

    /* The free frames start right after the kernel */
    let next_free_frame = Frame::get_frame_for(
                      _kernel_end
                    - (::arch::KERNEL_BASE as usize)
                    + 2 * page_size as usize,
                    page_size as usize);
    log!("Kernel ends at 0x{:x}, so the next free frame is at 0x{:x}",
         _kernel_end, next_free_frame.frame_addr());

    /* Start a basic frame allocation algorithm. */
    let ret = FrameAllocator {
        next_free_frame: Frame::get_frame_for(next_free_frame.frame_addr(),
                                              page_size as usize),
        page_size: page_size as usize,
    };
    ret
}
