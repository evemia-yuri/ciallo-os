use core::panic::PanicInfo;

use crate::{log_error, println};
use crate::sbi;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    if let Some(lo) = _info.location() {
        log_error!(
            "Panicked at: {}:{} {}",
            lo.file(),
            lo.line(),
            _info.message()
        );
    } else {
        log_error!("Panicked: {}", _info.message());
    }

    sbi::shutdown(true)
}
