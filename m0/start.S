/*
 * BL808 M0 (E907): power MM, release D0, then park. No stack or UART.
 * Boot ROM supplies clocks and Group0 XIP mapping at 0x58000000;
 * the image body lives at flash offset 0x2000.
 */

	.equ	CSR_MHCR, 0x7c1
	.equ	CSR_MHINT, 0x7c5
	.equ	MSTATUS_MIE, (1 << 3)
	.equ	MHCR_DE, (1 << 0)
	.equ	MHCR_WB, (1 << 1)
	.equ	MHCR_BPE, (1 << 5)
	.equ	MHCR_L0BTB, (1 << 12)

	.equ	MM_CLK_CTRL_CPU, 0x30007000
	.equ	MM_SW_SYS_RESET, 0x30007040
	.equ	PDS_CPU_CORE_CFG0, 0x2000e110
	.equ	GLB_SWRST_CFG2, 0x20000548
	.equ	PDS_CTL2, 0x2000e010

	.equ	D0_CLK_EN, (1 << 12)
	.equ	D0_RESET, (1 << 8)
	.equ	LP_CLK_EN, (1 << 28)
	.equ	LP_RESET, (1 << 3)
	.equ	MM_PWR_OFF, (1 << 1)
	.equ	MM_ISO_EN, (1 << 5)
	.equ	MM_GATE_CLK, (1 << 17)
	.equ	MM_MEM_STBY, (1 << 13)
	.equ	MM_PDS_RST, (1 << 9)

	.equ	CLK_SETTLE_LOOPS, 1024
	.equ	POWER_SETTLE_LOOPS, 65536

/* Ordered MMIO read-modify-write; clobbers t0, t1, t2. */
	.macro	mmio_clear	addr, mask
	li	t0, \addr
	lw	t1, 0(t0)
	li	t2, ~(\mask)
	and	t1, t1, t2
	sw	t1, 0(t0)
	fence	iorw, iorw
	.endm

	.macro	mmio_set	addr, mask
	li	t0, \addr
	lw	t1, 0(t0)
	li	t2, \mask
	or	t1, t1, t2
	sw	t1, 0(t0)
	fence	iorw, iorw
	.endm

/*
 * Conservative settling delays, not a calibrated microsecond timer.
 * At <= 1 GHz and >= 1 cycle/iteration, 1024 iterations exceed 1 us
 * and 65536 exceed 45 us for power settling. XIP stalls only lengthen
 * them. Clobbers t3; the caller must order MMIO before starting a delay.
 */
	.macro	delay count
	li	t3, \count
.Ldelay\@:
	addi	t3, t3, -1
	bnez	t3, .Ldelay\@
	.endm

	.section	.text._start, "ax", @progbits
	.balign	4
	.globl	_start
	.type	_start, @function
_start:
	csrci	mstatus, MSTATUS_MIE
	la	t0, .Lpark
	csrw	mtvec, t0

	/* Disable D-cache and speculative accesses during XIP bring-up. */
	li	t0, MHCR_DE | MHCR_WB | MHCR_BPE | MHCR_L0BTB
	csrc	CSR_MHCR, t0
	csrw	CSR_MHINT, zero

	/* Hold D0: clock off, settle, then assert core reset. */
	mmio_clear	MM_CLK_CTRL_CPU, D0_CLK_EN
	delay	CLK_SETTLE_LOOPS
	mmio_set	MM_SW_SYS_RESET, D0_RESET

	/* Keep the unused LP core in reset as well. */
	mmio_clear	PDS_CPU_CORE_CFG0, LP_CLK_EN
	delay	CLK_SETTLE_LOOPS
	mmio_set	GLB_SWRST_CFG2, LP_RESET

	/* MM supply on; settle before removing isolation. */
	mmio_clear	PDS_CTL2, MM_PWR_OFF
	delay	POWER_SETTLE_LOOPS
	mmio_clear	PDS_CTL2, MM_ISO_EN
	mmio_clear	PDS_CTL2, MM_GATE_CLK
	mmio_clear	PDS_CTL2, MM_MEM_STBY
	mmio_clear	PDS_CTL2, MM_PDS_RST

	/* Release D0: clock on, settle, then deassert core reset. */
	mmio_set	MM_CLK_CTRL_CPU, D0_CLK_EN
	delay	CLK_SETTLE_LOOPS
	mmio_clear	MM_SW_SYS_RESET, D0_RESET

	.balign	4
.Lpark:
	wfi
	j	.Lpark
	.size	_start, . - _start
