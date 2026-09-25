#![no_std]
#![no_main]

use core::arch::global_asm;

use kernel::init;

global_asm!(include_str!("entry.asm"));

#[unsafe(no_mangle)]
pub fn kernel_main() -> ! {
    init::clear_bss();

    use kernel::println;
    println!("Hello World!");

    // panic!("Shutting down!");

    loop {
        core::hint::spin_loop();
    }
}
