//! Memory-mapped register definitions and ordered access.

pub const MM_CLK_CTRL_CPU: usize = 0x3000_7000;
pub const MM_CLK_CTRL_PERI: usize = 0x3000_7010;
pub const UART_CLK_SEL_MASK: u32 = 3 << 4;
pub const UART_CLK_SEL_XCLK: u32 = 2 << 4;
pub const XCLK_SEL_XTAL: u32 = 1 << 10;
pub const UART_CLK_ENABLE: u32 = 1 << 16;
pub const UART_CLK_DIV_MASK: u32 = 7 << 17;

pub const GPIO16_CONFIG: usize = 0x2000_0904;
pub const GPIO17_CONFIG: usize = 0x2000_0908;
pub const GPIO_INPUT_ENABLE: u32 = 1 << 0;
pub const GPIO_SCHMITT_ENABLE: u32 = 1 << 1;
pub const GPIO_DRIVE_STRENGTH: u32 = 1 << 2;
pub const GPIO_PULL_UP: u32 = 1 << 4;
pub const GPIO_FUNC_MM_UART: u32 = 21 << 8;
pub const GPIO_INT_MASK: u32 = 1 << 22;

pub const LDO12UHS: usize = 0x200006d0;
pub const PHY_CFG_00: usize = 0x3000f100;
pub const PHY_CFG_04: usize = 0x3000f104;
pub const PHY_CFG_08: usize = 0x3000f108;
pub const PHY_CFG_0C: usize = 0x3000f10c;
pub const PHY_CFG_10: usize = 0x3000f110;
pub const PHY_CFG_14: usize = 0x3000f114;
pub const PHY_CFG_18: usize = 0x3000f118;
pub const PHY_CFG_1C: usize = 0x3000f11c;
pub const PHY_CFG_20: usize = 0x3000f120;
pub const PHY_CFG_24: usize = 0x3000f124;
pub const PHY_CFG_28: usize = 0x3000f128;
pub const PHY_CFG_2C: usize = 0x3000f12c;
pub const PHY_CFG_30: usize = 0x3000f130;
pub const PHY_CFG_34: usize = 0x3000f134;
pub const PHY_CFG_38: usize = 0x3000f138;
pub const PHY_CFG_3C: usize = 0x3000f13c;
pub const PHY_CFG_40: usize = 0x3000f140;
pub const PHY_CFG_44: usize = 0x3000f144;
pub const PHY_CFG_48: usize = 0x3000f148;
pub const PHY_CFG_4C: usize = 0x3000f14c;
pub const PHY_CFG_50: usize = 0x3000f150;
pub const TZC_PSRAMA_TZSRG_CTRL: usize = 0x20005380;
pub const UHS_AUTO_FRESH_1: usize = 0x3000f010;
pub const UHS_AUTO_FRESH_2: usize = 0x3000f014;
pub const UHS_AUTO_FRESH_4: usize = 0x3000f01c;
pub const UHS_BASIC: usize = 0x3000f000;
pub const UHS_CMD: usize = 0x3000f004;
pub const UHS_MANUAL: usize = 0x3000f00c;
pub const UHS_PLL_CFG0: usize = 0x200007d0;
pub const UHS_PLL_CFG1: usize = 0x200007d4;
pub const UHS_PLL_CFG4: usize = 0x200007e0;
pub const UHS_PLL_CFG5: usize = 0x200007e4;
pub const UHS_PLL_CFG6: usize = 0x200007e8;
pub const UHS_PLL_CFG9: usize = 0x200007f4;
pub const UHS_TIMING_CTRL: usize = 0x3000f030;

pub const AUPLL_FBDV_RSTB: Field = Field::new(2, 1);
pub const AUPLL_REFCLK_SEL: Field = Field::new(16, 2);
pub const AUPLL_REFDIV_RATIO: Field = Field::new(8, 4);
pub const AUPLL_SDMIN: Field = Field::new(0, 19);
pub const AUPLL_SDM_RSTB: Field = Field::new(0, 1);
pub const AUPLL_SEL_SAMPLE_CLK: Field = Field::new(0, 2);
pub const AUPLL_VCO_SPEED: Field = Field::new(0, 3);
pub const CEN_DLY_DRV: Field = Field::new(28, 4);
pub const CEN_SR: Field = Field::new(20, 2);
pub const CK_DLY_DRV: Field = Field::new(16, 4);
pub const CK_SR: Field = Field::new(8, 2);
pub const DM0_DLY_DRV: Field = Field::new(28, 4);
pub const DM0_SR: Field = Field::new(20, 2);
pub const DM1_DLY_DRV: Field = Field::new(12, 4);
pub const DM1_SR: Field = Field::new(4, 2);
pub const DQ0_DLY_DRV: Field = Field::new(28, 4);
pub const DQ0_DLY_RX: Field = Field::new(24, 4);
pub const DQ0_SR: Field = Field::new(16, 2);
pub const DQ10_DLY_DRV: Field = Field::new(28, 4);
pub const DQ10_DLY_RX: Field = Field::new(24, 4);
pub const DQ10_SR: Field = Field::new(16, 2);
pub const DQ11_DLY_DRV: Field = Field::new(12, 4);
pub const DQ11_DLY_RX: Field = Field::new(8, 4);
pub const DQ11_SR: Field = Field::new(0, 2);
pub const DQ12_DLY_DRV: Field = Field::new(28, 4);
pub const DQ12_DLY_RX: Field = Field::new(24, 4);
pub const DQ12_SR: Field = Field::new(16, 2);
pub const DQ13_DLY_DRV: Field = Field::new(12, 4);
pub const DQ13_DLY_RX: Field = Field::new(8, 4);
pub const DQ13_SR: Field = Field::new(0, 2);
pub const DQ14_DLY_DRV: Field = Field::new(28, 4);
pub const DQ14_DLY_RX: Field = Field::new(24, 4);
pub const DQ14_SR: Field = Field::new(16, 2);
pub const DQ15_DLY_DRV: Field = Field::new(12, 4);
pub const DQ15_DLY_RX: Field = Field::new(8, 4);
pub const DQ15_SR: Field = Field::new(0, 2);
pub const DQ1_DLY_DRV: Field = Field::new(12, 4);
pub const DQ1_DLY_RX: Field = Field::new(8, 4);
pub const DQ1_SR: Field = Field::new(0, 2);
pub const DQ2_DLY_DRV: Field = Field::new(28, 4);
pub const DQ2_DLY_RX: Field = Field::new(24, 4);
pub const DQ2_SR: Field = Field::new(16, 2);
pub const DQ3_DLY_DRV: Field = Field::new(12, 4);
pub const DQ3_DLY_RX: Field = Field::new(8, 4);
pub const DQ3_SR: Field = Field::new(0, 2);
pub const DQ4_DLY_DRV: Field = Field::new(28, 4);
pub const DQ4_DLY_RX: Field = Field::new(24, 4);
pub const DQ4_SR: Field = Field::new(16, 2);
pub const DQ5_DLY_DRV: Field = Field::new(12, 4);
pub const DQ5_DLY_RX: Field = Field::new(8, 4);
pub const DQ5_SR: Field = Field::new(0, 2);
pub const DQ6_DLY_DRV: Field = Field::new(28, 4);
pub const DQ6_DLY_RX: Field = Field::new(24, 4);
pub const DQ6_SR: Field = Field::new(16, 2);
pub const DQ7_DLY_DRV: Field = Field::new(12, 4);
pub const DQ7_DLY_RX: Field = Field::new(8, 4);
pub const DQ7_SR: Field = Field::new(0, 2);
pub const DQ8_DLY_DRV: Field = Field::new(28, 4);
pub const DQ8_DLY_RX: Field = Field::new(24, 4);
pub const DQ8_SR: Field = Field::new(16, 2);
pub const DQ9_DLY_DRV: Field = Field::new(12, 4);
pub const DQ9_DLY_RX: Field = Field::new(8, 4);
pub const DQ9_SR: Field = Field::new(0, 2);
pub const DQS0N_DLY_RX: Field = Field::new(8, 4);
pub const DQS0_DIFF_DLY_RX: Field = Field::new(28, 4);
pub const DQS0_DLY_DRV: Field = Field::new(24, 4);
pub const DQS0_DLY_RX: Field = Field::new(20, 4);
pub const DQS1N_DLY_RX: Field = Field::new(8, 4);
pub const DQS1_DIFF_DLY_RX: Field = Field::new(28, 4);
pub const DQS1_DLY_DRV: Field = Field::new(24, 4);
pub const DQS1_DLY_RX: Field = Field::new(20, 4);
pub const DQ_OE_DN_N_REG: Field = Field::new(20, 3);
pub const DQ_OE_DN_P_REG: Field = Field::new(16, 3);
pub const DQ_OE_MID_N_REG: Field = Field::new(12, 3);
pub const DQ_OE_MID_P_REG: Field = Field::new(8, 3);
pub const DQ_OE_UP_N_REG: Field = Field::new(4, 3);
pub const DQ_OE_UP_P_REG: Field = Field::new(0, 3);
pub const LDO12UHS_VOUT_SEL: Field = Field::new(20, 4);
pub const ODT_SEL: Field = Field::new(28, 4);
pub const ODT_SEL_HW: Field = Field::new(20, 1);
pub const OE_CTRL_HW: Field = Field::new(27, 1);
pub const OE_TIMER: Field = Field::new(24, 2);
pub const PHY_RL_ANA: Field = Field::new(20, 3);
pub const PHY_RL_DIG: Field = Field::new(16, 4);
pub const PHY_WL_ANA: Field = Field::new(12, 3);
pub const PHY_WL_CEN_ANA: Field = Field::new(24, 3);
pub const PHY_WL_DIG: Field = Field::new(8, 3);
pub const PHY_WL_DQ_ANA: Field = Field::new(4, 3);
pub const PHY_WL_DQ_DIG: Field = Field::new(0, 3);
pub const PSRAM_TYPE: Field = Field::new(8, 2);
pub const PU_AUPLL: Field = Field::new(10, 1);
pub const PU_AUPLL_SFREG: Field = Field::new(9, 1);
pub const PU_LDO12UHS: Field = Field::new(0, 1);
pub const REG_ADDRMB_MSK: Field = Field::new(16, 8);
pub const REG_AF_EN: Field = Field::new(1, 1);
pub const REG_BUST_CYCLE: Field = Field::new(0, 7);
pub const REG_CONFIG_REQ: Field = Field::new(2, 1);
pub const REG_GLBR_PULSE: Field = Field::new(0, 1);
pub const REG_INIT_EN: Field = Field::new(0, 1);
pub const REG_LINEAR_BND_B: Field = Field::new(28, 4);
pub const REG_PCK_T_DIV: Field = Field::new(24, 8);
pub const REG_REFI_CYCLE: Field = Field::new(0, 16);
pub const REG_UHS_DMY0: Field = Field::new(8, 8);
pub const REG_UHS_DMY1: Field = Field::new(16, 8);
pub const REG_WIN_CYCLE: Field = Field::new(0, 28);
pub const REG_WIN_REF_CNT: Field = Field::new(16, 13);
pub const UHSPLL_EVEN_DIV_EN: Field = Field::new(7, 1);
pub const UHSPLL_EVEN_DIV_RATIO: Field = Field::new(0, 7);
pub const UHSPLL_SSC_EN: Field = Field::new(0, 1);
pub const VREF_MODE: Field = Field::new(26, 1);

#[derive(Clone, Copy)]
pub struct Field {
    mask: u32,
    shift: u32,
}

impl Field {
    const fn new(shift: u32, width: u32) -> Self {
        Self {
            mask: ((1u32 << width) - 1) << shift,
            shift,
        }
    }

    const fn insert(self, value: u32, bits: u32) -> u32 {
        (value & !self.mask) | ((bits << self.shift) & self.mask)
    }
}

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

pub unsafe fn modify32(addr: usize, fields: &[(Field, u32)]) {
    let mut value = unsafe { read32(addr) };
    for &(field, bits) in fields {
        value = field.insert(value, bits);
    }
    unsafe { write32(addr, value) }
}

/// Conservative lower bound at a core clock no faster than 1 GHz.
pub fn delay_us(us: u32) {
    let count = us.saturating_mul(1024);
    if count != 0 {
        unsafe {
            core::arch::asm!(
                "2:", "addi {count}, {count}, -1", "bnez {count}, 2b",
                count = inout(reg) count => _, options(nostack, nomem)
            );
        }
    }
}
