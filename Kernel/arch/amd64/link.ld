ENTRY(start)
OUTPUT_FORMAT(elf64-x86-64)

KERNEL_BASE = 0xFFFFFFFF80000000;

SECTIONS {

    . = 0xA000;

    SMP_AP_START = .;
    .smp.text : AT(ADDR(.smp.text)) {
        KEEP(*(.smp.text))
    }
    SMP_AP_END = .;
	
	. = 0x100000;
	
	. += SIZEOF_HEADERS;
	
	.init : AT(ADDR(.init)) {
		KEEP( *(.multiboot) )
		*(.inittext)
	}

	. += KERNEL_BASE;
	
	.text ALIGN(0x1000) : AT(ADDR(.text) - KERNEL_BASE) {
		*(.text .text.*)
	}

	/* Read-write data, page aligned for the .padata section */
	.data ALIGN(0x1000) : AT(ADDR(.data) - KERNEL_BASE) {
		*(.padata)
		*(.data .data.*)
	}

	/* read-only data, page aligned to allow use of the no-execute feature */
	.rodata ALIGN(0x1000) : AT(ADDR(.rodata) - KERNEL_BASE) {
		*(.rodata .rodata.*)
	}
	
	/* Zero-initialised data */
	.bss : AT(ADDR(.bss) - KERNEL_BASE) {
		*(.bss .bss.*)
	}
	
	kernel_end = .;
	
	/DISCARD/ : {
		*(.note .note.*)
	}
}

