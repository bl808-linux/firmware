	.equ	CSR_MXSTATUS, 0x7c0
	.equ	CSR_MHCR, 0x7c1
	.equ	CSR_MHINT, 0x7c5
	.equ	MHCR_IE, (1 << 0)
	.equ	MHCR_DE, (1 << 1)
	.equ	MHCR_WB, (1 << 3)
	.equ	MHCR_BPE, (1 << 5)
	.equ	MHCR_L0BTB, (1 << 6)
	.equ	MSTATUS_IE, (1 << 3) | (1 << 1)
	.equ	MSTATUS_FS, (3 << 13)
	.equ	MSTATUS_FS_INITIAL, (1 << 13)
	.equ	MXSTATUS_THEADISAEE, (1 << 22)
	.equ	MXSTATUS_MM, (1 << 15)
	.equ	MM_MISC_VRAM_CTRL, 0x30000050
	.equ	SRAM_H2PF_RELEASE, (3 << 1)
	.equ	SRAM_VRAM_RELEASE, (1 << 4)
	.equ	SRAM_SUB_RELEASE, (1 << 6)
	.equ	SRAM_BLAI_RELEASE, (1 << 7)
	.equ	SRAM_COMMIT, (1 << 0)

	.section	.text._start, "ax", @progbits
	.balign	4
	.globl	_start
	.type	_start, @function
_start:
	csrci	mstatus, MSTATUS_IE
	csrw	mie, zero
	lla	t0, .Lpark
	csrw	mtvec, t0

	/* Use physical addresses and stop speculative data accesses. */
	csrw	satp, zero
	sfence.vma
	li	t0, MXSTATUS_THEADISAEE | MXSTATUS_MM
	csrs	CSR_MXSTATUS, t0
	li	t0, MHCR_DE | MHCR_WB | MHCR_BPE | MHCR_L0BTB
	csrc	CSR_MHCR, t0
	csrw	CSR_MHINT, zero
	fence	iorw, iorw

	/* Cache instructions fetched from flash. */
	fence.i
	csrsi	CSR_MHCR, MHCR_IE
	fence.i

	/* RV64GC permits floating-point instructions in Rust code. */
	li	t0, MSTATUS_FS
	csrc	mstatus, t0
	li	t0, MSTATUS_FS_INITIAL
	csrs	mstatus, t0
	csrwi	fcsr, 0

	/* Make shared SRAM writable before using the MM VRAM stack. */
	li	t0, MM_MISC_VRAM_CTRL
	lw	t1, 0(t0)
	ori	t1, t1, SRAM_H2PF_RELEASE | SRAM_VRAM_RELEASE
	ori	t1, t1, SRAM_SUB_RELEASE | SRAM_BLAI_RELEASE
	sw	t1, 0(t0)
	fence	iorw, iorw
	lw	t1, 0(t0)
	ori	t1, t1, SRAM_COMMIT
	sw	t1, 0(t0)
	fence	iorw, iorw

	.option	push
	.option	norelax
	lla	gp, __global_pointer$
	.option	pop
	lla	sp, _stack_top

	/* Copy initialized data to MM VRAM. */
	lla	t0, __data_load
	lla	t1, __data_start
	lla	t2, __data_end
.Lcopy_data:
	bgeu	t1, t2, .Lclear_bss
	ld	t3, 0(t0)
	sd	t3, 0(t1)
	addi	t0, t0, 8
	addi	t1, t1, 8
	j	.Lcopy_data

.Lclear_bss:
	lla	t0, __bss_start
	lla	t1, __bss_end
.Lzero_bss:
	bgeu	t0, t1, .Lenter_rust
	sd	zero, 0(t0)
	addi	t0, t0, 8
	j	.Lzero_bss

.Lenter_rust:
	call	main

	.balign	4
.Lpark:
	wfi
	j	.Lpark
	.size	_start, . - _start
