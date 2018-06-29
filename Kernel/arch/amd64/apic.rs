#[path = "../../logging.rs"]
mod logging;

pub struct LAPIC {
    lapic_addr: usize, /* Address of the LAPIC from this CPU */
    lapic_id: u8, /* The CPU's LAPIC id */
}

impl LAPIC {

    pub fn new(fma: &mut ::mm::pmm::FrameAllocator, lapic_addr: usize, lapic_id: u8) -> LAPIC {
        ::mm::vmm::map_addr_current(fma, lapic_addr, lapic_addr);
        let ret = LAPIC {
            lapic_addr: lapic_addr,
            lapic_id: lapic_id,
        };
        ret.write_u32(0xF0, ret.read_u32(0xF0) | 0x100);
        ret
    }

    fn read_u32(&self, register: isize) -> u32 {
        let lapic_ptr: *const u32 = (self.lapic_addr + register as usize) as *const u32;
        unsafe { *lapic_ptr }
    }

    fn write_u32(&self, register: isize, value: u32) {
        let lapic_ptr: *mut u32 = (self.lapic_addr + register as usize) as *mut u32;
        log!("lapic_ptr 0x{:016x}, register: 0x{:x}, value 0x{:x}",
                lapic_ptr as usize, register, value);
        unsafe { *lapic_ptr = value };
    }

    /* Wait for the current LAPIC to clear any pending IPI */
    fn wait_for_pending_ipi(&self) {
        loop {
            if (self.read_u32(0x30)) & (1 << 12) == 0 {
                break;
            }
        }
    }

    pub fn send_ipi_to(&self, target_id: u8, vector: u8) {
        let mut control: u32 = vector as u32;
        self.wait_for_pending_ipi();

        self.write_u32(0x31 * 0x10, (target_id as u32) << 24);

        control = control | (1 << 14);

        self.write_u32(0x30 * 0x10, control);
    }

    pub fn send_sipi_to(&self, target_id: u8, vector: u8) {
        let higher: u32 = (target_id as u32) << 24;
        let lower: u32 = (6 << 8) | (1 << 14) | (vector as u32);

        self.write_u32(0x31 * 0x10, higher);
        self.write_u32(0x30 * 0x10, lower);
    }

    pub fn send_init_to(&self, target_id: u8) {
        let higher: u32 = (target_id as u32) << 24;
        let lower: u32 = (5 << 8);

        self.write_u32(0x31 * 0x10, higher);
        self.write_u32(0x30 * 0x10, lower);
    }
}
