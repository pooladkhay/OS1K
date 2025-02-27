ENTRY(boot)

SECTIONS {
    . = 0x80200000;
    __kernel_base = .;

    .text :{
        KEEP(*(.text.boot));
        . = ALIGN(4);
        KEEP(*(.text.trap_entry))
        *(.text .text.*);
    }

    .rodata : ALIGN(4) {
        *(.rodata .rodata.*);
    }

    .data : ALIGN(4) {
        *(.data .data.*);
    }

    .bss : ALIGN(4) {
        __bss = .;
        *(.bss .bss.* .sbss .sbss.*);
        __bss_end = .;
    }

    . = ALIGN(4);
    . += 128 * 1024; /* 128KB */
    __stack_top = .;

    . = ALIGN(4096);
    __allocator_mem = .;
    . += 512 * 1024; /* 512KB */
    __allocator_mem_end = .;

    . = ALIGN(4096);
    __free_ram = .;
    /* memset on ram panics if > 125mb */
    . += 64 * 1024 * 1024; /* 65MB */
    __free_ram_end = .;
}
