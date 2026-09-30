OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;
    start_kernel = .;

    start_text = .;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }

    . = ALIGN(4k);
    end_text = .;
    start_rodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.start_rodata .start_rodata.*)
    }

    . = ALIGN(4k);
    end_rodata = .;
    start_data = .;
    .data : {
        *(.data .data.*)
        *(.start_data .start_data.*)
    }

    . = ALIGN(4k);
    end_data = .;
    .bss : {
        *(.bss.stack)
        start_bss = .;
        *(.bss .bss.*)
        *(.start_bss .start_bss.*)
    }

    . = ALIGN(4k);
    end_bss = .;
    end_kernel = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}
