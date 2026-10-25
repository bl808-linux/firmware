//! M0 register definitions and ordered memory-mapped access.

pub const MM_CLK_CTRL_CPU: usize = 0x3000_7000;
pub const MM_SW_SYS_RESET: usize = 0x3000_7040;
pub const PDS_CTL2: usize = 0x2000_e010;

pub const D0_CLK_EN: u32 = 1 << 12;
pub const D0_RESET: u32 = 1 << 8;
pub const MM_PWR_OFF: u32 = 1 << 1;
pub const MM_ISO_EN: u32 = 1 << 5;
pub const MM_GATE_CLK: u32 = 1 << 17;
pub const MM_MEM_STBY: u32 = 1 << 13;
pub const MM_PDS_RST: u32 = 1 << 9;

pub unsafe fn read32(addr: usize) -> u32 {
    unsafe { (addr as *const u32).read_volatile() }
}

pub unsafe fn write32(addr: usize, value: u32) {
    unsafe {
        (addr as *mut u32).write_volatile(value);
        core::arch::asm!("fence iorw, iorw", options(nostack));
    }
}

pub unsafe fn update32(addr: usize, clear: u32, set: u32) {
    unsafe { write32(addr, (read32(addr) & !clear) | set) }
}

/// Preserve the startup's conservative loop counts; not a calibrated timer.
pub fn delay(count: u32) {
    if count != 0 {
        unsafe {
            core::arch::asm!(
                "2:", "addi {count}, {count}, -1", "bnez {count}, 2b",
                count = inout(reg) count => _, options(nostack, nomem)
            );
        }
    }
}

pub unsafe fn read16(addr: usize) -> u16 {
    unsafe { (addr as *const u16).read_volatile() }
}
pub unsafe fn write16(addr: usize, value: u16) {
    unsafe {
        (addr as *mut u16).write_volatile(value);
        core::arch::asm!("fence iorw, iorw", options(nostack));
    }
}

pub unsafe fn write8(addr: usize, value: u8) {
    unsafe {
        (addr as *mut u8).write_volatile(value);
        core::arch::asm!("fence iorw, iorw", options(nostack));
    }
}
