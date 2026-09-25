use core::ptr::{addr_of_mut, write_bytes};

/// Clears the `.bss` section to zero.
///
/// The linker must provide `start_bss` and `end_bss` symbols
/// such that `[start_bss, end_bss)` is a valid and writable memory range.
pub fn clear_bss() {
    // These symbols are provided by linker script.
    // They mark the half-open range [start_bss, end_bss).
    unsafe extern "C" {
        static mut start_bss: [u8; 0];
        static mut end_bss: [u8; 0];
    }

    unsafe {
        let start_ptr: *mut u8 = addr_of_mut!(start_bss).cast();
        let end_ptr: *mut u8 = addr_of_mut!(end_bss).cast();

        // We always assume the linker script is reliable.
        // If it is not, crash and raise an error as soon as possible.
        let size = end_ptr.addr() - start_ptr.addr();
        write_bytes(start_ptr, 0, size);
    }
}
