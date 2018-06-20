#[path="../../logging.rs"]
mod logging;

use ::arch::PAGE_SHIFT;

pub fn remap_kernel()
{
    let remap_target = ::arch::KERNEL_BASE as usize;

    log!("Attempting to remap the kernel to 0x{:x}, page 0x{:x}",
            remap_target, remap_target >> PAGE_SHIFT);
}
