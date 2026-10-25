use core::ptr::addr_of;

use crate::config::PHYS_MEMORY_END;
use crate::mm::address::PhysAddr;
use crate::mm::frame_allocator::FrameAllocatorError;

pub mod address;
pub mod frame_allocator;
pub mod heap_alloc;
pub mod page_table;

/// Read the linker-provided kernel end and initialize the global allocator.
///
/// The linker script must export an `end_kernel` symbol. The symbol is only
/// used as an address; no memory is read through it.
pub fn init_frame_allocator() -> Result<(), FrameAllocatorError> {
    unsafe extern "C" {
        static end_kernel: [u8; 0];
    }

    let ptr: *const u8 = addr_of!(end_kernel).cast();

    let start = PhysAddr::try_from(ptr.addr())?.ceil()?;
    let end = PhysAddr::try_from(PHYS_MEMORY_END)?.floor();
    if start > end {
        Err(FrameAllocatorError::InvalidRange)
    } else {
        frame_allocator::init_frame_allocator(start..end)
    }
}
