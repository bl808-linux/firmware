//! Copy the boot images while the loader remains in flash and MM VRAM.

use crate::image::{self, Error};

pub const OPENSBI_ENTRY: usize = 0x3ef8_0000;
pub const DTB_ADDR: usize = 0x51ff_8000;
const SRAM_SIZE: usize = 512 * 1024;
const OPENSBI_SRC: usize = 0x5880_0000;
const UBOOT_SRC: usize = 0x580a_0000;
const UBOOT_ADDR: usize = 0x5000_0000;
const UBOOT_SLOT_SIZE: usize = 0x460000;
const DTB_SRC: usize = 0x5808_0000;
const DTB_SLOT_SIZE: usize = 64 * 1024;

// The fixed XIP slots are readable memory; the caller supplies a checked size.
unsafe fn flash(addr: usize, len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(addr as *const u8, len) }
}

unsafe fn copy(src: usize, dst: usize, len: usize) {
    unsafe {
        core::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, len);
        core::arch::asm!("fence rw, rw", options(nostack));
    }
}

pub fn load() -> Result<(), Error> {
    unsafe {
        let uboot_len = image::uboot_length(flash(UBOOT_SRC - 8, 8), UBOOT_SLOT_SIZE)?;
        image::validate_uboot(flash(UBOOT_SRC, 4))?;
        let fdt_len = image::fdt_length(flash(DTB_SRC, 40), DTB_SLOT_SIZE)?;
        let sbi_len = image::opensbi_length(flash(OPENSBI_SRC, 128), SRAM_SIZE)?;
        copy(UBOOT_SRC, UBOOT_ADDR, uboot_len);
        copy(DTB_SRC, DTB_ADDR, fdt_len);
        copy(OPENSBI_SRC, OPENSBI_ENTRY, sbi_len);
    }
    Ok(())
}

/// Enter M-mode firmware; the next stage establishes its own stack and gp.
pub fn enter() -> ! {
    unsafe {
        core::arch::asm!(
            "fence rw, rw",
            "fence.i",
            "jr {entry}",
            entry = in(reg) OPENSBI_ENTRY,
            in("a0") 0usize,
            in("a1") DTB_ADDR,
            in("a2") 0usize,
            options(noreturn)
        );
    }
}
