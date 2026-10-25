/* Boot ROM enters Group0 at the XIP base; body offset remains 0x2000. */

	.equ	CSR_MHCR, 0x7c1
	.equ	MSTATUS_MIE, (1 << 3)
	.equ	MHCR_IE, (1 << 0)
	.equ	MHCR_DE, (1 << 1)

	.equ	MM_CLK_CTRL_CPU, 0x30007000
	.equ	MM_SW_SYS_RESET, 0x30007040
	.equ	D0_CLK_EN, (1 << 12)
	.equ	D0_RESET, (1 << 8)

	.equ	CLK_SETTLE_LOOPS, 1024

	/* Ordered MMIO read-modify-write; clobbers t0, t1, t2. */
	.macro	mmio_clear addr, mask
	li	t0, \addr
	lw	t1, 0(t0)
	li	t2, ~(\mask)
	and	t1, t1, t2
	sw	t1, 0(t0)
	fence	iorw, iorw
	.endm

	.macro	mmio_set addr, mask
	li	t0, \addr
	lw	t1, 0(t0)
	li	t2, \mask
	or	t1, t1, t2
	sw	t1, 0(t0)
	fence	iorw, iorw
	.endm

	/* Uncalibrated clock-settling delay; clobbers t3. */
	.macro	settle
	li	t3, CLK_SETTLE_LOOPS
.Lsettle\@:
	addi	t3, t3, -1
	bnez	t3, .Lsettle\@
	.endm

	/* Bootstrap address loads must not depend on the ROM's gp value. */
	.option	push
	.option	norelax
	.section	.text._start, "ax", @progbits
	.balign	4
	.globl	_start
	.type	_start, @function
_start:
	csrci	mstatus, MSTATUS_MIE
	csrw	mie, zero
	lla	t0, .Lboot_park
	csrw	mtvec, t0

	/* Disable I/D caches before relocating the runtime and using DMA. */
	li	t0, MHCR_IE | MHCR_DE
	csrc	CSR_MHCR, t0

	/* Hold D0 before copying into MCU SRAM. */
	mmio_clear MM_CLK_CTRL_CPU, D0_CLK_EN
	settle
	mmio_set MM_SW_SYS_RESET, D0_RESET

	/* Copy code, constants and initialized data using their Flash LMA. */
	lla	t0, __runtime_load
	lla	t1, __runtime_start
	lla	t2, __runtime_end
.Lcopy_runtime:
	bgeu	t1, t2, .Lclear_bss
	lw	t3, 0(t0)
	sw	t3, 0(t1)
	addi	t0, t0, 4
	addi	t1, t1, 4
	j	.Lcopy_runtime

.Lclear_bss:
	lla	t0, __bss_start
	lla	t1, __bss_end
.Lzero_bss:
	bgeu	t0, t1, .Lhandoff
	sw	zero, 0(t0)
	addi	t0, t0, 4
	j	.Lzero_bss

.Lhandoff:
	fence	rw, rw
	fence.i
	lla	gp, __global_pointer$
	lla	sp, _stack_top
	lla	t0, _trap
	csrw	mtvec, t0
	lla	t0, main
	jr	t0

	.balign	4
.Lboot_park:
	wfi
	j	.Lboot_park
	.size	_start, . - _start
	.option	pop

	/* Trap handling after relocation must not fetch instructions from NOR. */
	.section	.text.trap, "ax", @progbits
	.balign	4
	.globl	_trap
	.type	_trap, @function
_trap:
	wfi
	j	_trap
	.size	_trap, . - _trap
