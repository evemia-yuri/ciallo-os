use alloc::vec::Vec;
use core::ops::Range;

use spin::Mutex;

use crate::mm::address::{AddressError, PhysPageNum};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FrameAllocatorError {
    InvalidRange,
    NotInitialized,
    AlreadyInitialized,
    OutOfMemory,
    DoubleFree(PhysPageNum),
    FreeUnallocated(PhysPageNum),

    Address(AddressError),
}

impl From<AddressError> for FrameAllocatorError {
    fn from(ae: AddressError) -> Self {
        Self::Address(ae)
    }
}

/// Owns one physical frame. The `Drop` implementation should be connected to
/// the global allocator only after that allocator is protected by a lock.
pub struct FrameTracker {
    ppn: PhysPageNum,
}

impl FrameTracker {
    fn new(ppn: PhysPageNum) -> Self {
        Self { ppn }
    }

    pub fn ppn(&self) -> PhysPageNum {
        self.ppn
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        // Drop cannot return a Result. Reaching this branch indicates an
        // allocator invariant violation, so fail loudly instead of silently
        // leaking or double-freeing the frame.
        if let Err(error) = dealloc_frame(self.ppn) {
            panic!(
                "[kernel] Failed to deallocate frame {:?}: {:?}",
                self.ppn, error
            );
        }
    }
}

trait FrameAllocator {
    fn alloc(&mut self) -> Option<FrameTracker>;
    fn dealloc(&mut self, ppn: PhysPageNum) -> Result<(), FrameAllocatorError>;
}

pub struct StackFrameAllocator {
    range_start: PhysPageNum,
    current: PhysPageNum,
    end: PhysPageNum,
    recycled: Vec<PhysPageNum>,
}

impl StackFrameAllocator {
    pub fn new(range: Range<PhysPageNum>) -> Result<Self, FrameAllocatorError> {
        if range.start > range.end {
            Err(FrameAllocatorError::InvalidRange)
        } else {
            Ok(Self {
                range_start: range.start,
                current: range.start,
                end: range.end,
                recycled: Vec::new(),
            })
        }
    }

    pub fn contains_allocated(&self, ppn: PhysPageNum) -> bool {
        self.range_start <= ppn && ppn < self.current && !self.recycled.contains(&ppn)
    }
}

impl FrameAllocator for StackFrameAllocator {
    fn alloc(&mut self) -> Option<FrameTracker> {
        if let Some(ppn) = self.recycled.pop() {
            Some(FrameTracker::new(ppn))
        } else {
            if self.current >= self.end {
                None
            } else {
                let ppn = self.current;
                self.current = self
                    .current
                    .checked_add(1)
                    .expect("[kernel] Frame range must not overflow the PPN range.");
                Some(FrameTracker::new(ppn))
            }
        }
    }

    fn dealloc(&mut self, ppn: PhysPageNum) -> Result<(), FrameAllocatorError> {
        if self.contains_allocated(ppn) {
            self.recycled.push(ppn);
            Ok(())
        } else {
            if self.recycled.contains(&ppn) {
                Err(FrameAllocatorError::DoubleFree(ppn))
            } else {
                Err(FrameAllocatorError::FreeUnallocated(ppn))
            }
        }
    }
}

// `Mutex::new` is const, so a separate lazy-static initialization layer is
// unnecessary. `None` means the kernel has not supplied a physical range yet.
static FRAME_ALLOCATOR: Mutex<Option<StackFrameAllocator>> = Mutex::new(None);

pub(super) fn init_frame_allocator(range: Range<PhysPageNum>) -> Result<(), FrameAllocatorError> {
    let mut global = FRAME_ALLOCATOR.lock();
    if global.is_some() {
        Err(FrameAllocatorError::AlreadyInitialized)
    } else {
        *global = Some(StackFrameAllocator::new(range)?);
        Ok(())
    }
}

pub fn alloc_frame() -> Result<Option<FrameTracker>, FrameAllocatorError> {
    let mut global = FRAME_ALLOCATOR.lock();
    let allocator = global.as_mut().ok_or(FrameAllocatorError::NotInitialized)?;
    Ok(allocator.alloc())
}

fn dealloc_frame(ppn: PhysPageNum) -> Result<(), FrameAllocatorError> {
    let mut global = FRAME_ALLOCATOR.lock();
    let allocator = global.as_mut().ok_or(FrameAllocatorError::NotInitialized)?;
    allocator.dealloc(ppn)
}
