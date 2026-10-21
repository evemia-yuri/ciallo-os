#![no_std]
#![no_main]

use core::arch::global_asm;

use kernel::init;
use kernel::mm::heap_alloc;
use kernel::println;
use kernel::trap;

global_asm!(include_str!("init/entry.asm"));

#[unsafe(no_mangle)]
pub fn kernel_main() -> ! {
    init::clear_bss();
    trap::init_trap();
    heap_alloc::init_heap();

    println!("Hello World!");
    // panic!("Shutting down!");

    loop {
        core::hint::spin_loop();
    }
}
