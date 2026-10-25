//! D0 time counter (CPU_RTC) initialization before OpenSBI consumes it.

use crate::regs::{read32, write32};

pub const CPU_RTC: usize = 0x3000_0018;
pub const RTC_ENABLE: u32 = 1 << 31;
pub const RTC_RESET: u32 = 1 << 30;
pub const RTC_DIV_MASK: u32 = 0x3ff;
/// Divisor is encoded as value + 1. 379 gives the vendor's 380 divide.
pub const RTC_DIV: u32 = 379;

/// Follow the SDK disable/update/enable sequence while preserving the
/// unrelated control bits. The reset bit is left released.
pub fn init() {
    unsafe {
        let mut value = read32(CPU_RTC);

        value &= !(RTC_ENABLE | RTC_RESET);
        write32(CPU_RTC, value);

        value = (value & !RTC_DIV_MASK) | RTC_DIV;
        write32(CPU_RTC, value);

        value |= RTC_ENABLE;
        write32(CPU_RTC, value);

        debug_assert_eq!(
            read32(CPU_RTC) & (RTC_ENABLE | RTC_RESET | RTC_DIV_MASK),
            RTC_ENABLE | RTC_DIV
        );
    }
}
