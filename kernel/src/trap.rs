use core::arch::global_asm;
use core::ptr::addr_of_mut;

use riscv::register::stvec::{self, Stvec, TrapMode};

pub mod context;

global_asm!(include_str!("trap/trap.asm"));

pub fn init_trap() {
    unsafe extern "C" {
        static mut __alltraps: [u8; 0];
    }

    unsafe {
        let __alltraps_ptr: *mut u8 = addr_of_mut!(__alltraps).cast();
        stvec::write(Stvec::new(__alltraps_ptr.addr(), TrapMode::Direct))
    }
}
