use sbi_rt::{self, NoReason, Shutdown, SystemFailure};

pub fn console_put_byte(byte: u8) {
    sbi_rt::console_write_byte(byte);
}

pub fn shutdown(failure: bool) -> ! {
    if failure {
        sbi_rt::system_reset(Shutdown, SystemFailure);
    } else {
        sbi_rt::system_reset(Shutdown, NoReason);
    }
    unreachable!();
}
