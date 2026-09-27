use sbi_rt::legacy;
use sbi_rt::{self, NoReason, Shutdown, SystemFailure};

pub fn console_putchar(ch: usize) {
    // TODO: Replace the function which is deprecated.
    #[allow(deprecated)]
    legacy::console_putchar(ch);
}

pub fn shutdown(failure: bool) -> ! {
    if !failure {
        sbi_rt::system_reset(Shutdown, NoReason);
    } else {
        sbi_rt::system_reset(Shutdown, SystemFailure);
    }
    unreachable!();
}
