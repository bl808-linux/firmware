//! Board clocks and pin routing required before entering OpenSBI.

use crate::regs::*;

pub fn init() {
    unsafe {
        // Supply UART3 with the 40 MHz crystal clock, divider one.
        update32(MM_CLK_CTRL_PERI, UART_CLK_ENABLE, 0);
        update32(
            MM_CLK_CTRL_CPU,
            UART_CLK_SEL_MASK,
            UART_CLK_SEL_XCLK | XCLK_SEL_XTAL,
        );
        update32(MM_CLK_CTRL_PERI, UART_CLK_DIV_MASK, UART_CLK_ENABLE);

        // The OpenSBI UART driver also relies on the GPIO16/17 pin routing.
        let pads = GPIO_FUNC_MM_UART | GPIO_SCHMITT_ENABLE | GPIO_INT_MASK;
        write32(GPIO16_CONFIG, pads);
        write32(
            GPIO17_CONFIG,
            pads | GPIO_INPUT_ENABLE | GPIO_DRIVE_STRENGTH | GPIO_PULL_UP,
        );
    }
}
