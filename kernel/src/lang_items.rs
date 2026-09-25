use core::panic::PanicInfo;

use crate::sbi;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    if let Some(lo) = _info.location() {
        println!(
            "Panicked at: {}:{} {}",
            lo.file(),
            lo.line(),
            _info.message()
        );
    } else {
        println!("Panicked: {}", _info.message());
    }

    sbi::shutdown(true)
}
