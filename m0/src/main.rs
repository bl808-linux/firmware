//! M0 startup: run from MCU SRAM, power MM and release D0.

#![no_std]
#![no_main]

mod board;
mod dfu;
mod flash;
mod regs;
mod sd;
mod storage;
mod uart;
mod usb;

core::arch::global_asm!(include_str!("start.S"));

fn park() -> ! {
    loop {
        unsafe { core::arch::asm!("wfi", options(nomem, nostack)) }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    uart::log(format_args!("M0 panic: {info}\n"));
    park()
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> ! {
    let clock = uart::init();
    uart::log(format_args!(
        "\nBL808 M0: UART0 {} 8N1, clock {clock} Hz\n",
        uart::BAUD
    ));
    board::init();
    uart::log(format_args!("M0: D0 released\n"));
    if let Some(mut usb) = usb::Device::init() {
        uart::log(format_args!("M0: USB DFU ready, protocol 2\n"));
        loop {
            usb.poll();
        }
    }
    uart::log(format_args!("M0: USB initialization failed\n"));
    park()
}
