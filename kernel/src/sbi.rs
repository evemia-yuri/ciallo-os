use sbi_rt::legacy;
use sbi_rt::{NoReason, Shutdown, SystemFailure, system_reset};

pub fn console_putchar(ch: usize) {
    // TODO: Replace the function which is deprecated.
    #[allow(deprecated)]
    legacy::console_putchar(ch);
}

pub fn shutdown(failure: bool) -> ! {
    if !failure {
        system_reset(Shutdown, NoReason);
    } else {
        system_reset(Shutdown, SystemFailure);
    }
    unreachable!();
}
