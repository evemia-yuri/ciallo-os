#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

mod batch;
mod config;
mod console;
mod init;
mod log;
mod mm;
mod panic;
mod sbi;
mod trap;

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {
    log_info!("[test] Running {} tests", tests.len());
    for test in tests {
        test();
    }
    log_info!("[test] Tests finished, exiting...");
    sbi::shutdown(false);
}

#[unsafe(no_mangle)]
pub fn kernel_main() -> ! {
    init::clear_bss();

    trap::init_trap();
    mm::heap_alloc::init_heap();
    mm::init_frame_allocator().unwrap();

    log_info!("Kernel running...");

    #[cfg(test)]
    test_main();

    loop {
        core::hint::spin_loop();
    }
}
