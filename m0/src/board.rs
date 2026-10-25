//! MM power-up and D0 release, after startup has held D0 in reset.

use crate::regs::*;

pub fn init() {
    unsafe {
        update32(PDS_CTL2, MM_PWR_OFF, 0);
        delay(65536);
        update32(PDS_CTL2, MM_ISO_EN, 0);
        update32(PDS_CTL2, MM_GATE_CLK, 0);
        update32(PDS_CTL2, MM_MEM_STBY, 0);
        update32(PDS_CTL2, MM_PDS_RST, 0);

        update32(MM_CLK_CTRL_CPU, 0, D0_CLK_EN);
        delay(1024);
        update32(MM_SW_SYS_RESET, D0_RESET, 0);
    }
}

/// Enter maintenance without touching the MCU USB clock or MM power supply.
pub fn hold_d0() {
    unsafe {
        update32(MM_CLK_CTRL_CPU, D0_CLK_EN, 0);
        delay(1024);
        update32(MM_SW_SYS_RESET, 0, D0_RESET);
    }
}

pub fn d0_is_held() -> bool {
    unsafe { read32(MM_SW_SYS_RESET) & D0_RESET != 0 && read32(MM_CLK_CTRL_CPU) & D0_CLK_EN == 0 }
}

/// Software POR, entirely in SRAM. Return clock roots to RC32M before reset.
pub fn reboot() -> ! {
    crate::uart::log(format_args!("M0: software POR\n"));
    hold_d0();
    unsafe {
        // MM XCLK=RC32M, root=XCLK, CPU and BCLK2X dividers zero.
        update32(MM_CLK_CTRL_CPU, (1 << 10) | (1 << 11), 0);
        delay(1024);
        update32(0x3000_7004, 0x00ff_00ff, 0);
        update32(MM_CLK_CTRL_CPU, 0, 1 << 18);
        wait_set(MM_CLK_CTRL_CPU, 1 << 20);

        // MCU XCLK=RC32M, root=XCLK; HCLK/BCLK and LP dividers zero.
        update32(0x2000_f030, 3, 0);
        delay(1024);
        update32(0x2000_0090, 0x00ff_ff00, 0);
        update32(0x2000_0094, 0, 1);
        wait_set(0x2000_0094, 1 << 2);
        update32(0x2000_e12c, 0xff, 0);
        update32(0x2000_0094, 0, 1 << 16);
        wait_set(0x2000_0094, 1 << 18);

        // GLB_SWRST_CFG2.reg_ctrl_pwron_rst, low -> high -> low.
        update32(0x2000_0548, 1, 0);
        update32(0x2000_0548, 0, 1);
        update32(0x2000_0548, 1, 0);
    }
    loop {
        unsafe { core::arch::asm!("wfi", options(nomem, nostack)) }
    }
}

unsafe fn wait_set(address: usize, mask: u32) {
    for _ in 0..1024 {
        if unsafe { read32(address) } & mask != 0 {
            break;
        }
    }
}
