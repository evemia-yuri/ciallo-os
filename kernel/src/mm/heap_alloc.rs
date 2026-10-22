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

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use core::ptr::addr_of_mut;

    use crate::config::KERNEL_HEAP_SIZE;
    use crate::mm::heap_alloc::HEAP_SPACE;

    #[test_case]
    fn test_heap_basic() {
        let num = Box::new(42);
        assert_eq!(*num, 42);

        let num_ptr = &*num as *const i32 as usize;
        let heap_start = unsafe { addr_of_mut!(HEAP_SPACE.0) as usize };
        let heap_end = heap_start + KERNEL_HEAP_SIZE;

        assert!(
            num_ptr >= heap_start && num_ptr < heap_end,
            "Box pointer is out of heap bounds. ptr: {:#x}, heap: {:#x}..{:#x}",
            num_ptr,
            heap_start,
            heap_end
        );

        let mut vec = Vec::new();
        let end = 1024;
        for i in 0..end {
            vec.push(i)
        }

        for i in 0..end {
            assert_eq!(vec[i], i);
        }

        let vec_ptr = vec.as_ptr() as usize;
        assert!(
            vec_ptr >= heap_start && vec_ptr < heap_end,
            "Vec pointer out of heap bounds."
        );
    }

    #[test_case]
    fn test_heap_free_and_reuse() {
        let loop_cnt = 1024;
        let size = 1024;
        for _ in 0..loop_cnt {
            let mut v: Vec<u8> = Vec::with_capacity(size); // 每次分配 1KB
            v.push(1);
        }
    }
}
