//! D0 boot loader: prepare PSRAM and enter OpenSBI from internal SRAM.

#![no_std]
#![no_main]

mod board;
mod boot;
mod image;
mod psram;
mod regs;

core::arch::global_asm!(include_str!("start.S"));

fn park() -> ! {
    loop {
        unsafe { core::arch::asm!("wfi", options(nomem, nostack)) }
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    park()
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> ! {
    psram::init();
    if boot::load().is_err() {
        park();
    }
    board::init();
    boot::enter()
}
