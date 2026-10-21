use riscv::register::sstatus::{self, SPP, Sstatus};

#[repr(C)]
pub struct TrapContext {
    pub x: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
}

impl TrapContext {
    /// Build trap context for a new task.
    ///
    /// # Arguments
    /// * `entry` - Entry address of application.
    /// * `sp` - The address of the top of the user stack.
    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);

        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
        };
        cx.set_sp(sp);
        cx
    }

    /// Set the stack pointer in the current trap context.
    /// In the risc-v arch, the `x[2]` register is used as the stack pointer.
    ///
    /// # Arguments
    /// * `sp` - The target stack pointer address.
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }
}
