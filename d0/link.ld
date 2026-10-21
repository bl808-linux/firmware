OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY
{
	flash (rx) : ORIGIN = 0x58000000, LENGTH = 320K
	data (rw) : ORIGIN = 0x3f000000, LENGTH = 16K
	stack (rw) : ORIGIN = 0x3f004000, LENGTH = 16K
}

SECTIONS
{
	.text :
	{
		KEEP(*(.text._start))
		*(.text .text.*)
	} > flash

	.rodata : { *(.rodata .rodata.*) } > flash
	.data : ALIGN(8)
	{
		__data_start = .;
		PROVIDE(__global_pointer$ = . + 0x800);
		*(.sdata .sdata.* .data .data.* .got .got.*)
		. = ALIGN(8);
		__data_end = .;
	} > data AT> flash
	__data_load = LOADADDR(.data);

	.bss (NOLOAD) : ALIGN(8)
	{
		__bss_start = .;
		*(.sbss .sbss.* .bss .bss.*)
		*(COMMON)
		. = ALIGN(8);
		__bss_end = .;
	} > data

	_stack_bottom = ORIGIN(stack);
	_stack_top = ORIGIN(stack) + LENGTH(stack);
	/DISCARD/ : { *(.eh_frame .eh_frame.*) }

	ASSERT((__data_load & 7) == 0, "D0 data load must be aligned")
}
