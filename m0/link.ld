/* Group0 XIP entry; image body at flash offset 0x2000. */
OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY
{
	flash (rx) : ORIGIN = 0x58000000, LENGTH = 32M
}

SECTIONS
{
	.text :
	{
		KEEP(*(.text._start))
		*(.text .text.*)
	} > flash

	.rodata : { *(.rodata .rodata.*) } > flash
	.data : { *(.data .data.* .sdata .sdata.* .got .got.*) } > flash
	.bss (NOLOAD) : { *(.bss .bss.* .sbss .sbss.*) *(COMMON) } > flash

	.riscv.attributes 0 : { *(.riscv.attributes) }
	/DISCARD/ : { *(.comment .note.GNU-stack) }

	ASSERT(_start == ORIGIN(flash), "M0 entry must be 0x58000000")
	ASSERT(SIZEOF(.data) == 0 && SIZEOF(.bss) == 0, "M0 must not use RAM data")
}
