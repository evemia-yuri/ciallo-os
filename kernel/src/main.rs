#![no_std]
#![no_main]

use core::arch::global_asm;

global_asm!(include_str!("entry.asm"));

#[unsafe(no_mangle)]
pub fn kernel_main() -> ! {
    clear_bss();

    use kernel::println;
    println!("Hello World!");

    // panic!("Shutting down!");

    loop {
        core::hint::spin_loop();
    }
}

macro_rules! linker_symbol_addr {
    ($symbol:path) => {
        ($symbol as *const ()).addr()
    };
}

fn clear_bss() {
    unsafe extern "C" {
        safe fn start_bss();
        safe fn end_bss();
    }

    (linker_symbol_addr!(start_bss)..linker_symbol_addr!(end_bss))
        .for_each(|x| unsafe { (x as *mut u8).write_volatile(0) })
}
