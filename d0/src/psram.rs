//! Initialize the board's 64 MiB UHS PSRAM at 1400 Mbps.

use crate::regs::*;

fn pll_init() {
    unsafe {
        modify32(UHS_PLL_CFG0, &[(PU_AUPLL, 0)]);
        modify32(UHS_PLL_CFG0, &[(PU_AUPLL_SFREG, 0)]);
        modify32(
            UHS_PLL_CFG1,
            &[(AUPLL_REFCLK_SEL, 0), (AUPLL_REFDIV_RATIO, 2)],
        );
        modify32(UHS_PLL_CFG4, &[(AUPLL_SEL_SAMPLE_CLK, 2)]);
        modify32(UHS_PLL_CFG5, &[(AUPLL_VCO_SPEED, 4)]);
        modify32(
            UHS_PLL_CFG1,
            &[(UHSPLL_EVEN_DIV_EN, 1), (UHSPLL_EVEN_DIV_RATIO, 28)],
        );
        // Fractional multiplier: 1400 Mbps with a 40 MHz / 2 reference clock.
        modify32(UHS_PLL_CFG6, &[(AUPLL_SDMIN, 143360)]);
        modify32(UHS_PLL_CFG0, &[(PU_AUPLL_SFREG, 1)]);
        delay_us(3);
        modify32(UHS_PLL_CFG0, &[(PU_AUPLL, 1)]);
        delay_us(3);
        for field in [AUPLL_SDM_RSTB, AUPLL_FBDV_RSTB] {
            for value in [1, 0, 1] {
                modify32(UHS_PLL_CFG0, &[(field, value)]);
                delay_us(2);
            }
        }
        delay_us(45);
        modify32(UHS_PLL_CFG9, &[(UHSPLL_SSC_EN, 0)]);
    }
}
fn analog_init() {
    unsafe {
        modify32(LDO12UHS, &[(PU_LDO12UHS, 1)]);
        delay_us(300);
        modify32(LDO12UHS, &[(LDO12UHS_VOUT_SEL, 6)]);
        delay_us(1);
        modify32(PHY_CFG_50, &[(DQ_OE_MID_N_REG, 0), (DQ_OE_MID_P_REG, 0)]);
        modify32(PHY_CFG_40, &[(REG_UHS_DMY1, 0xfc), (REG_UHS_DMY0, 1)]);
        delay_us(1);
        for (i, value) in [
            0x802b0200, 0x60206020, 0x70027002, 0x70027002, 0x70027002, 0x70027002, 0x70027002,
            0x70027002, 0x70027002, 0x70027002, 0x26000000, 0x26000006,
        ]
        .into_iter()
        .enumerate()
        {
            write32(PHY_CFG_00 + i * 4, value);
        }
        update32(PHY_CFG_30, !0x08ff_ffff, 0x0700_0000);
        update32(PHY_CFG_48, 0x300, 0x200);
        update32(PHY_CFG_4C, 0x1f_0000, 0);
        update32(PHY_CFG_50, !0xff88_ff88, 0x0033_0033);
        update32(PHY_CFG_40, 0x30_0000, 0);
        delay_us(1);
        modify32(PHY_CFG_40, &[(REG_UHS_DMY1, 0xcf), (REG_UHS_DMY0, 0)]);
        delay_us(1);
        modify32(PHY_CFG_50, &[(DQ_OE_MID_N_REG, 3), (DQ_OE_MID_P_REG, 3)]);
        delay_us(150);
    }
}
fn controller_init() {
    unsafe {
        write32(UHS_TIMING_CTRL, 0x1202_000b);
        analog_init();
        modify32(UHS_MANUAL, &[(REG_PCK_T_DIV, 2)]);
        write32(UHS_AUTO_FRESH_1, 0x16e360);
        modify32(UHS_AUTO_FRESH_2, &[(REG_REFI_CYCLE, 370)]);
        modify32(UHS_AUTO_FRESH_4, &[(REG_BUST_CYCLE, 5)]);
        modify32(
            UHS_BASIC,
            &[
                (REG_AF_EN, 1),
                (REG_ADDRMB_MSK, 0x3f),
                (REG_LINEAR_BND_B, 0xb),
            ],
        );
        modify32(UHS_BASIC, &[(REG_INIT_EN, 1)]);
        pad_init();
        modify32(LDO12UHS, &[(LDO12UHS_VOUT_SEL, 5)]);
        delay_us(250);
    }
}
fn phy_init() {
    unsafe {
        modify32(
            PHY_CFG_30,
            &[(ODT_SEL, 0), (OE_CTRL_HW, 1), (VREF_MODE, 1), (OE_TIMER, 3)],
        );
        for (addr, value) in [
            (PHY_CFG_34, 0x09020303),
            (PHY_CFG_38, 0x040c0313),
            (PHY_CFG_3C, 0x07d11515),
            (PHY_CFG_44, 0x060f050c),
        ] {
            write32(addr, value);
        }
        modify32(PHY_CFG_50, &[(PHY_WL_CEN_ANA, 1)]);
        modify32(
            PHY_CFG_30,
            &[
                (PHY_RL_ANA, 2),
                (PHY_RL_DIG, 7),
                (PHY_WL_ANA, 1),
                (PHY_WL_DIG, 2),
                (PHY_WL_DQ_ANA, 2),
                (PHY_WL_DQ_DIG, 2),
            ],
        );
        delay_us(100);
        modify32(UHS_BASIC, &[(REG_CONFIG_REQ, 1)]);
        delay_us(10);
        modify32(UHS_CMD, &[(REG_GLBR_PULSE, 1)]);
        delay_us(100);
        modify32(UHS_BASIC, &[(REG_CONFIG_REQ, 0)]);
        delay_us(100);
        // Auto refresh for 64 MiB at 1400 Mbps, normal temperature.
        modify32(UHS_BASIC, &[(REG_AF_EN, 0)]);
        modify32(UHS_MANUAL, &[(REG_PCK_T_DIV, 4)]);
        modify32(UHS_AUTO_FRESH_1, &[(REG_WIN_CYCLE, 0x155cbf)]);
        modify32(
            UHS_AUTO_FRESH_2,
            &[(REG_WIN_REF_CNT, 0x2000), (REG_REFI_CYCLE, 0xa9)],
        );
        modify32(UHS_AUTO_FRESH_4, &[(REG_BUST_CYCLE, 4)]);
        modify32(UHS_BASIC, &[(REG_AF_EN, 1)]);
        delay_us(50);
    }
}
pub fn init() {
    pll_init();
    controller_init();
    phy_init();
    // Remove PSRAMA region barriers only after the controller is ready.
    unsafe { update32(TZC_PSRAMA_TZSRG_CTRL, 0x7_0000, 0) }
}
fn pad_init() {
    unsafe {
        modify32(PHY_CFG_48, &[(PSRAM_TYPE, 2)]);
        modify32(PHY_CFG_4C, &[(ODT_SEL_HW, 0x0)]);
        modify32(PHY_CFG_30, &[(ODT_SEL, 0x0), (VREF_MODE, 0x1)]);
        modify32(PHY_CFG_00, &[(CEN_DLY_DRV, 0x8), (CK_DLY_DRV, 0xB)]);
        modify32(PHY_CFG_04, &[(DM0_DLY_DRV, 0x7), (DM1_DLY_DRV, 0x7)]);
        modify32(PHY_CFG_24, &[(DQ15_DLY_DRV, 0x7), (DQ14_DLY_DRV, 0x7)]);
        modify32(PHY_CFG_20, &[(DQ13_DLY_DRV, 0x7), (DQ12_DLY_DRV, 0x7)]);
        modify32(PHY_CFG_1C, &[(DQ11_DLY_DRV, 0x7), (DQ10_DLY_DRV, 0x7)]);
        modify32(PHY_CFG_18, &[(DQ9_DLY_DRV, 0x7), (DQ8_DLY_DRV, 0x7)]);
        modify32(PHY_CFG_14, &[(DQ7_DLY_DRV, 0x7), (DQ6_DLY_DRV, 0x7)]);
        modify32(PHY_CFG_10, &[(DQ5_DLY_DRV, 0x7), (DQ4_DLY_DRV, 0x7)]);
        modify32(PHY_CFG_0C, &[(DQ3_DLY_DRV, 0x7), (DQ2_DLY_DRV, 0x7)]);
        modify32(PHY_CFG_08, &[(DQ1_DLY_DRV, 0x7), (DQ0_DLY_DRV, 0x7)]);
        modify32(PHY_CFG_28, &[(DQS0_DLY_DRV, 0x6)]);
        modify32(PHY_CFG_2C, &[(DQS1_DLY_DRV, 0x6)]);
        modify32(PHY_CFG_30, &[(OE_TIMER, 0x3)]);
        modify32(PHY_CFG_00, &[(CEN_SR, 0x2), (CK_SR, 0x2)]);
        modify32(PHY_CFG_04, &[(DM1_SR, 0x2), (DM0_SR, 0x2)]);
        modify32(PHY_CFG_24, &[(DQ15_SR, 0x2), (DQ14_SR, 0x2)]);
        modify32(PHY_CFG_20, &[(DQ13_SR, 0x2), (DQ12_SR, 0x2)]);
        modify32(PHY_CFG_1C, &[(DQ11_SR, 0x2), (DQ10_SR, 0x2)]);
        modify32(PHY_CFG_18, &[(DQ9_SR, 0x2), (DQ8_SR, 0x2)]);
        modify32(PHY_CFG_14, &[(DQ7_SR, 0x2), (DQ6_SR, 0x2)]);
        modify32(PHY_CFG_10, &[(DQ5_SR, 0x2), (DQ4_SR, 0x2)]);
        modify32(PHY_CFG_0C, &[(DQ3_SR, 0x2), (DQ2_SR, 0x2)]);
        modify32(PHY_CFG_08, &[(DQ1_SR, 0x2), (DQ0_SR, 0x2)]);
        modify32(
            PHY_CFG_50,
            &[
                (DQ_OE_DN_P_REG, 0x3),
                (DQ_OE_DN_N_REG, 0x3),
                (DQ_OE_UP_P_REG, 0x3),
                (DQ_OE_UP_N_REG, 0x3),
            ],
        );
        modify32(PHY_CFG_24, &[(DQ15_DLY_RX, 0x0), (DQ14_DLY_RX, 0x0)]);
        modify32(PHY_CFG_20, &[(DQ13_DLY_RX, 0x0), (DQ12_DLY_RX, 0x0)]);
        modify32(PHY_CFG_1C, &[(DQ11_DLY_RX, 0x0), (DQ10_DLY_RX, 0x0)]);
        modify32(PHY_CFG_18, &[(DQ9_DLY_RX, 0x0), (DQ8_DLY_RX, 0x0)]);
        modify32(PHY_CFG_14, &[(DQ7_DLY_RX, 0x0), (DQ6_DLY_RX, 0x0)]);
        modify32(PHY_CFG_10, &[(DQ5_DLY_RX, 0x0), (DQ4_DLY_RX, 0x0)]);
        modify32(PHY_CFG_0C, &[(DQ3_DLY_RX, 0x0), (DQ2_DLY_RX, 0x0)]);
        modify32(PHY_CFG_08, &[(DQ1_DLY_RX, 0x0), (DQ0_DLY_RX, 0x0)]);
        modify32(
            PHY_CFG_28,
            &[
                (DQS0_DLY_RX, 0x0),
                (DQS0N_DLY_RX, 0x0),
                (DQS0_DIFF_DLY_RX, 0x3),
            ],
        );
        modify32(
            PHY_CFG_2C,
            &[
                (DQS1_DLY_RX, 0x0),
                (DQS1N_DLY_RX, 0x0),
                (DQS1_DIFF_DLY_RX, 0x3),
            ],
        );
        delay_us(200);
    }
}
