/* Group0 XIP entry; image body at flash offset 0x2000. */
OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY
{
	flash (rx) : ORIGIN = 0x58000000, LENGTH = 320K
	ram (rwx) : ORIGIN = 0x62028000, LENGTH = 28K
	stack (rw) : ORIGIN = 0x6202f000, LENGTH = 4K
}

SECTIONS
{
	.boot :
	{
		KEEP(*(.text._start))
		. = ALIGN(4);
	} > flash

	.runtime : ALIGN(4)
	{
		__runtime_start = .;
		*(.text .text.*)
		*(.rodata .rodata.* .srodata .srodata.*)
		. = ALIGN(4);
		PROVIDE(__global_pointer$ = . + 0x800);
		*(.data .data.* .sdata .sdata.* .got .got.*)
		. = ALIGN(4);
		__runtime_end = .;
	} > ram AT> flash
	__runtime_load = LOADADDR(.runtime);

	.bss (NOLOAD) : ALIGN(4)
	{
		__bss_start = .;
		*(.bss .bss.* .sbss .sbss.*)
		*(COMMON)
		. = ALIGN(4);
		__bss_end = .;
	} > ram

	_stack_bottom = ORIGIN(stack);
	_stack_top = ORIGIN(stack) + LENGTH(stack);
	__flash_end = __runtime_load + SIZEOF(.runtime);

	.riscv.attributes 0 : { *(.riscv.attributes) }
	/DISCARD/ : { *(.eh_frame .eh_frame.* .comment .note.GNU-stack) }

	ASSERT(_start == ORIGIN(flash), "M0 entry must be 0x58000000")
	ASSERT((__runtime_load & 3) == 0, "M0 Flash source must be word-aligned")
	ASSERT((__runtime_start & 3) == 0 && (__runtime_end & 3) == 0,
		"M0 SRAM copy must be word-aligned")
	ASSERT((__bss_start & 3) == 0 && (__bss_end & 3) == 0,
		"M0 BSS must be word-aligned")
	ASSERT(__bss_end <= _stack_bottom, "M0 runtime overlaps its stack")
	ASSERT((_stack_top & 15) == 0, "M0 stack must be ABI-aligned")
	ASSERT(__flash_end <= ORIGIN(flash) + LENGTH(flash), "M0 exceeds its Flash slot")
}
