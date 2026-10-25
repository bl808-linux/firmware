//! M1s Dock's M0 debug channel: UART0, GPIO14 TX / GPIO15 RX, 115200 8N1.

use crate::regs::*;
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};

const UART: usize = 0x2000_a000;
const TX_CONFIG: usize = UART;
const RX_CONFIG: usize = UART + 0x04;
const BIT_PERIOD: usize = UART + 0x08;
const DATA_CONFIG: usize = UART + 0x0c;
const SW_MODE: usize = UART + 0x1c;
const INT_MASK: usize = UART + 0x24;
const INT_ENABLE: usize = UART + 0x2c;
const STATUS: usize = UART + 0x30;
const FIFO_CONFIG: usize = UART + 0x80;
const FIFO_COUNTS: usize = UART + 0x84;
const FIFO_WRITE: usize = UART + 0x88;
const TX_FREE_MASK: u32 = 0x3f;
const TX_FIFO_SIZE: u32 = 32;
const TX_BUSY: u32 = 1;
const UART_CLOCK: usize = 0x2000_0150;
const UART_SIGNALS: usize = 0x2000_0154;
const CLOCK_GATE: usize = 0x2000_0584;
const HBN_GLB: usize = 0x2000_f030;
const GPIO14: usize = 0x2000_08fc;
const GPIO15: usize = 0x2000_0900;
const POLL_LIMIT: usize = 65536;
pub const BAUD: u32 = 115200;
static READY: AtomicBool = AtomicBool::new(false);

pub fn init() -> u32 {
    unsafe {
        // UART0's bus gate only; preserve USB and the other peripherals.
        update32(CLOCK_GATE, 0, 1 << 16);
        update32(UART_CLOCK, 1 << 4, 0);
        // UART0..2 XCLK source, divider one. Do not change the MCU clock root.
        update32(HBN_GLB, (1 << 15) | (1 << 2), 1 << 15);
        update32(UART_CLOCK, 7, 1 << 4);
        let clock = if read32(HBN_GLB) & 1 != 0 {
            40_000_000
        } else {
            32_000_000
        };
        // BL808 GPIO UART signal index is pin % 12: GPIO14/15 -> signal 2/3.
        update32(UART_SIGNALS, 0xff << 8, (2 << 8) | (3 << 12));
        let pads = (7 << 8) | (1 << 22) | (1 << 4) | (1 << 2) | (1 << 1) | 1;
        write32(GPIO14, pads);
        write32(GPIO15, pads);
        write32(TX_CONFIG, 0);
        write32(RX_CONFIG, 0);
        write32(INT_MASK, u32::MAX);
        write32(INT_ENABLE, 0);
        write32(FIFO_CONFIG, (1 << 2) | (1 << 3)); // Clear FIFOs, no DMA.
        write32(DATA_CONFIG, 0);
        write32(SW_MODE, 0);
        let period = (clock + BAUD / 2) / BAUD - 1;
        write32(BIT_PERIOD, period | (period << 16));
        write32(RX_CONFIG, (7 << 8) | 1);
        write32(TX_CONFIG, (7 << 8) | (1 << 11) | (1 << 2) | 1);
        READY.store(true, Ordering::Release);
        clock
    }
}

fn wait(ready: impl Fn() -> bool) -> fmt::Result {
    for _ in 0..POLL_LIMIT {
        if ready() {
            return Ok(());
        }
        core::hint::spin_loop();
    }
    Err(fmt::Error)
}

struct Console;

impl Write for Console {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for byte in text.bytes() {
            if byte == b'\n' {
                self.put(b'\r')?;
            }
            self.put(byte)?;
        }
        Ok(())
    }
}

impl Console {
    fn put(&mut self, byte: u8) -> fmt::Result {
        wait(|| unsafe { read32(FIFO_COUNTS) & TX_FREE_MASK != 0 })?;
        unsafe { write8(FIFO_WRITE, byte) };
        Ok(())
    }
}

/// One polled M0 context owns output; UART interrupts stay disabled.
pub fn log(args: fmt::Arguments<'_>) {
    if !READY.load(Ordering::Acquire) {
        return;
    }
    if Console.write_fmt(args).is_ok() {
        // Finish the record before board reset can switch the MCU clock root.
        let _ = wait(|| unsafe {
            read32(FIFO_COUNTS) & TX_FREE_MASK == TX_FIFO_SIZE && read32(STATUS) & TX_BUSY == 0
        });
    }
}
