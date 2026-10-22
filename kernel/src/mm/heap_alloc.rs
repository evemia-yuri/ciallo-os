use core::ptr::addr_of_mut;

use buddy_system_allocator::LockedHeap;

use crate::config::KERNEL_HEAP_SIZE;
use crate::{log_info, println};

#[repr(align(4096))]
struct HeapSpace([u8; KERNEL_HEAP_SIZE]);
static mut HEAP_SPACE: HeapSpace = HeapSpace([0; KERNEL_HEAP_SIZE]);

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

pub fn init_heap() {
    unsafe {
        let heap_start = addr_of_mut!(HEAP_SPACE.0) as usize;
        HEAP_ALLOCATOR.lock().init(heap_start, KERNEL_HEAP_SIZE);
    }
    log_info!("[kernel] Heap allocator initialized.");
}
