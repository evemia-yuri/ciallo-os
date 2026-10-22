use core::arch::global_asm;
use core::ptr::addr_of_mut;

use riscv::interrupt::{Exception, Interrupt, Trap};
use riscv::register::stvec::{self, Stvec, TrapMode};
use riscv::register::{scause, stval};

use crate::{log_info, log_warning, println};
use crate::trap::context::TrapContext;

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
    log_info!("[kernel] Trap handler initialized.");
}

#[unsafe(no_mangle)]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();
    let trap: Trap<Interrupt, Exception> = match scause.cause().try_into() {
        Ok(trap) => trap,
        Err(_) => panic!(
            "Unsupported trap {:?}. stval: {:#x}.",
            scause.cause(),
            stval
        ),
    };
    match trap {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            log_warning!(
                "[kernel] Page fault in application. address: {:#x}, instruction: {:#x}.",
                stval, cx.sepc
            );
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            log_warning!("[kernel] Illegal instruction in application. Core dumped.");
        }
        _ => {
            panic!(
                "Unsupported trap {:?}. stval: {:#x}, sepc: {:#x}.",
                trap, stval, cx.sepc
            );
        }
    }
    cx
}
