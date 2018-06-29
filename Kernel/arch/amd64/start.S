/*
 * Rust BareBones OS
 * - By John Hodge (Mutabah/thePowersGang) 
 *
 * arcm/amd64/start.S
 * - AMD64 Entrypoint
 *
 * == LICENCE ==
 * This code has been put into the public domain, there are no restrictions on
 * its use, and the author takes no liability.
 */

/* The kernel is linked to run at -2GB. This allows efficient addressing */
KERNEL_BASE = 0xFFFFFFFF80000000

/* === Multiboot Header === */
MULTIBOOT_PAGE_ALIGN  =  (1<<0)
MULTIBOOT_MEMORY_INFO =  (1<<1)
MULTIBOOT_REQVIDMODE  =  (1<<2)
MULTIBOOT_HEADER_MAGIC =  0x1BADB002
MULTIBOOT_HEADER_FLAGS = (MULTIBOOT_PAGE_ALIGN | MULTIBOOT_MEMORY_INFO | MULTIBOOT_REQVIDMODE)
MULTIBOOT_CHECKSUM     = -(MULTIBOOT_HEADER_MAGIC + MULTIBOOT_HEADER_FLAGS)
.section .multiboot, "a"
.globl mboot
mboot:
	.long MULTIBOOT_HEADER_MAGIC
	.long MULTIBOOT_HEADER_FLAGS
	.long MULTIBOOT_CHECKSUM
	.long mboot
	/* a.out kludge (not used, the kernel is elf) */
	.long 0, 0, 0, 0	/* load_addr, load_end_addr, bss_end_addr, entry_addr */
	/* Video mode */
	.long 0 	/* Mode type (0: LFB) */
	.long 0 	/* Width (no preference) */
	.long 0 	/* Height (no preference) */
	.long 32	/* Depth (32-bit preferred) */

#define DEBUG(c)	mov $0x3f8, %dx ; mov $c, %al ; outb %al, %dx

/* === Code === */
.section .inittext, "ax"
.globl start
.code32
start:
	/* The kernel starts in protected mode (32-bit mode, we want to switch to long mode) */
	
	/* 1. Save multiboot state */
	mov %eax, mboot_sig - KERNEL_BASE
	mov %ebx, mboot_ptr - KERNEL_BASE
	
	/* 2. Ensure that the CPU support long mode */
	mov $0x80000000, %eax
	cpuid
	/* - Check if CPUID supports the field we want to query */
	cmp $0x80000001, %eax
	jbe not64bitCapable
	/* - Test the IA-32e bit */
	mov $0x80000001, %eax
	cpuid
	test $0x20000000, %edx /* bit 29 = */
	jz not64bitCapable
	
	/* 3. Set up state for long mode */
	/* Enable:
	    PGE (Page Global Enable)
	  + PAE (Physical Address Extension)
	  + PSE (Page Size Extensions)
	*/
	mov %cr4, %eax
	or $(0x80|0x20|0x10), %eax
	mov %eax, %cr4

	/* Load PDP4 */
	mov $(init_pml4 - KERNEL_BASE), %eax
	mov %eax, %cr3

	/* Enable IA-32e mode (Also enables SYSCALL and NX) */
	mov $0xC0000080, %ecx
	rdmsr
	or $(1 << 11)|(1 << 8)|(1 << 0), %eax     /* NXE, LME, SCE */
	wrmsr

	/* Enable paging and enter long mode */
	mov %cr0, %eax
	or $0x80010000, %eax      /* PG & WP */
	mov %eax, %cr0
	lgdt GDTPtr_low - KERNEL_BASE
	ljmp $0x08, $start64


not64bitCapable:
	/* If the CPU isn't 64-bit capable, print a message to serial/b8000 then busy wait */
	mov $0x3f8, %dx
	mov $'N', %al ; outb %al, %dx
	movw $0x100|'N', 0xb8000
	mov $'o', %al ; outb %al, %dx
	movw $0x100|'o', 0xb8002
	mov $'t', %al ; outb %al, %dx
	movw $0x100|'t', 0xb8004
	mov $'6', %al ; outb %al, %dx
	movw $0x100|'6', 0xb8006
	mov $'4', %al ; outb %al, %dx
	movw $0x100|'4', 0xb8008
	
not64bitCapable.loop:
	hlt
	jmp not64bitCapable.loop
.code64
.globl start64
start64:
	/* Running in 64-bit mode, jump to high memory */
	lgdt GDTPtr
	mov $start64_high, %rax
	jmp *%rax

.code64
.globl start64_ap
start64_ap:
    //mov $0x3f8, %dx ; mov $'H', %al ; outb %al, %dx
    /* This an AP running in 64-bit mode, grab a unique stack */
    lgdt GDTPtr
    //mov $0x3f8, %dx ; mov $'E', %al ; outb %al, %dx
    mov $start64_ap_high, %rax
    jmp *%rax

.code64
.globl start64_ap_high
start64_ap_high:
    /* Full blown 64-bit mode now on the AP */
    //mov $0x3f8, %dx ; mov $'R', %al ; outb %al, %dx
    
    /* Get a unique stack */
1:
    movl unique_stack_id, %eax
    movl %eax, %ebx
    incl %ebx
    lock cmpxchgl %ebx, unique_stack_id
    jnz 1b

    /* Found a unique stack */
    mov %ebx, %esp
    inc %esp
    shl $8, %esp
    addq $ap_stack_base, %rsp

    //mov $0x3f8, %dx ; mov $'E', %al ; outb %al, %dx

    mov $0x10, %ax
    mov %ax, %ss
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs

    call kmain_ap
    

.section .inittext
.extern kmain
.globl start64_high
start64_high:
	/* and clear low-memory mapping */
	mov $0, %rax
	//mov %rax, init_pml4 - KERNEL_BASE + 0
	
	/* Set up segment registers */
	mov $0x10, %ax
	mov %ax, %ss
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs
	
	/* Set up stack pointer */
	mov $init_stack, %rsp
	
	/* call the rust code */
	call kmain

	/* and if that returns (it shouldn't) loop forever */
start64.loop:
	hlt
	jmp start64.loop

/*
 RDI = Destination
 RSI = Value
 RDX = Count
*/
//.section .text.memset
//.globl memset
//memset:
	//mov %rsi, %rax
	//mov %rdx, %rcx
	//rep stosb
	//ret
/*
 RDI = Destination
 RSI = Source
 RDX = Count
*/
//.section .text.memcpy
//.globl memcpy
//memcpy:
	//mov %rdx, %rcx
	//rep movsb
	//ret

/* APs start here */
.section .smp.text, "ax"
.globl smp_ap_boot
.code16
smp_ap_boot:
    nop
    nop
    //movw $0x01, smp_ap_booted
    jmp 1f

_32GDTPtr_low:
	.word _32GDTEND - _32GDT - 1
	.long _32GDT - KERNEL_BASE
_32GDT:
        .long 0x0, 0x0
        .long 0x0000FFFF, 0x00CF9A00    /* 0x08: 32-bit Kernel Code */
        .long 0x0000FFFF, 0x00CF9200    /* 0x10: 32-bit Kernel Data */
_32GDTEND:
1:
    cli
    cld
    mov $0, %ax
    mov %ax, %ss
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs

    in $0x92, %al
    or $0x02, %al
    out %al, $0x92

    lgdt _32GDTPtr_low

    //mov $0x3f8, %dx ; mov $'H', %al ; outb %al, %dx

    /* Enter protected mode */
    mov %cr0, %eax
    or $1, %eax
    mov %eax, %cr0

    //mov $0x3f8, %dx ; mov $'E', %al ; outb %al, %dx


    /* Load 32-bit CS */
    ljmp $0x08, $__smp_ap_boot_32
.code32
__smp_ap_boot_32:
    /* We are in full 32-bit protected mode now, let's setup PAE */
    //mov $0x3f8, %dx ; mov $'L', %al ; outb %al, %dx

    movw $0x10, %ax
    movw %ax, %ss
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %fs
    movw %ax, %gs

    /* enable PAE */
    movl %cr4, %eax
    orl $(1 << 5), %eax
    movl %eax, %cr4

    //mov $0x3f8, %dx ; mov $'L', %al ; outb %al, %dx

    /* load the temporary PML4 */
	mov $(init_pml4 - KERNEL_BASE), %eax
	mov %eax, %cr3
    //mov $0x3f8, %dx ; mov $'O', %al ; outb %al, %dx

    /* enable long mode */
	mov $0xC0000080, %ecx
	rdmsr
	or $(1 << 11)|(1 << 8)|(1 << 0), %eax     /* NXE, LME, SCE */
	wrmsr

    //mov $0x3f8, %dx ; mov $'_', %al ; outb %al, %dx

	/* Enable paging and enter long mode */
	mov %cr0, %eax
	or $0x80010000, %eax      /* PG & WP */
	mov %eax, %cr0

    //mov $0x3f8, %dx ; mov $'T', %al ; outb %al, %dx

	lgdt GDTPtr_low - KERNEL_BASE
	ljmp $0x08, $start64_ap

/* === Page-aligned data === */
.section .padata
/* Initial paging structures, four levels */
/* The +3 for sub-pages indicates "present (1) + writable (2)" */
.globl init_pml4
init_pml4:
	.quad low_pdpt - KERNEL_BASE + 3	/* low map for startup, will be cleared before rust code runs */
	.rept 512 - 3
		.quad 0
	.endr
	.quad 0 	/* If you so wish, this is a good place for the "Fractal" mapping */
	.quad init_pdpt - KERNEL_BASE + 3	/* Final mapping */
low_pdpt:
	.quad init_pd - KERNEL_BASE + 3	/* early init identity map */
	.rept 512 - 1
		.quad 0
	.endr
init_pdpt:	/* covers the top 512GB, 1GB each entry */
	.rept 512 - 2
		.quad 0
	.endr
	.quad init_pd - KERNEL_BASE + 3	/* at -2GB, identity map the kernel image */
	.quad 0
init_pd:
	/* 0x80 = Page size extension */
	.quad 0x000000 + 0x80 + 3	/* Map 2MB, enough for a 1MB kernel */
	.quad 0x200000 + 0x80 + 3	/* - give it another 2MB, just in case */
	.rept 512 - 2
		.quad 0
	.endr 
init_stack_base:
	.rept 0x1000 * 2
		.byte 0
	.endr
init_stack:

/* === General Data === */
.section .data
.globl mboot_sig
.globl mboot_ptr
.globl unique_stack_id
mboot_sig:	.long 0
mboot_ptr:	.long 0
unique_stack_id: .long 0

ap_stack_base:
	.rept 256 * 4
		.byte 0
	.endr
ap_stack:

/* Global Descriptor Table */
GDTPtr_low:
	.word GDTEnd - GDT - 1
	.long GDT - KERNEL_BASE
GDTPtr:
	.word GDTEnd - GDT - 1
	.quad GDT
.globl GDT
GDT:
	.long 0, 0
        .long 0x00000000, 0x00209A00	/* 0x08: 64-bit Code */
        .long 0x00000000, 0x00009200    /* 0x10: 64-bit Data */
        .long 0x00000000, 0x0040FA00    /* 0x18: 32-bit User Code */
        .long 0x00000000, 0x0040F200    /* 0x20: User Data        */
        .long 0x00000000, 0x0020FA00    /* 0x28: 64-bit User Code       */
        .long 0x00000000, 0x0000F200    /* 0x30: User Data (64 version) */
        .long 0x0000FFFF, 0x00CF9A00    /* 0x38: 32-bit Kernel Code */
        .long 0x0000FFFF, 0x00CF9200    /* 0x40: 32-bit Kernel Data */
GDTEnd:
