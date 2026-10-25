use alloc::vec;
use alloc::vec::Vec;

use bitflags::bitflags;

use crate::mm::address::{AddressError, PhysAddr, PhysPageNum, VirtPageNum};
use crate::mm::frame_allocator::{self, FrameAllocatorError, FrameTracker};

/// Each page-table level has 512 entries because its index is 9 bits wide.
const SV39_PAGE_TABLE_ENTRY_COUNT: usize = 512;
/// Sv39 page tables have levels 2, 1, and 0.
const SV39_PAGE_TABLE_LEVELS: usize = 3;

/// In a 64-bit Sv39 PTE, the 44-bit PPN field starts at bit 10.
const SV39_PTE_PPN_SHIFT: usize = 10;
const SV39_PTE_PPN_BITS: usize = 44;
/// Mask selecting the 44 PPN bits after shifting them down to bit zero.
const SV39_PTE_PPN_MASK: usize = (1 << SV39_PTE_PPN_BITS) - 1;
/// The low 8 PTE bits are V/R/W/X/U/G/A/D; higher reserved bits are excluded.
const SV39_PTE_FLAGS_MASK: usize = (1 << 8) - 1;

bitflags! {
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct PteFlags: usize {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct PageTableEntry {
    bits: usize,
}

impl PageTableEntry {
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub fn new(ppn: PhysPageNum, flags: PteFlags) -> Self {
        Self {
            bits: (ppn.get() << SV39_PTE_PPN_SHIFT) | flags.bits(),
        }
    }

    pub const fn bits(self) -> usize {
        self.bits
    }

    pub fn ppn(self) -> Result<PhysPageNum, AddressError> {
        let raw_ppn = (self.bits >> SV39_PTE_PPN_SHIFT) & SV39_PTE_PPN_MASK;
        raw_ppn.try_into()
    }

    pub fn flags(self) -> PteFlags {
        PteFlags::from_bits_retain(self.bits & SV39_PTE_FLAGS_MASK)
    }

    pub fn is_valid(self) -> bool {
        self.flags().contains(PteFlags::V)
    }

    pub fn is_leaf(self) -> bool {
        self.flags()
            .intersects(PteFlags::R | PteFlags::W | PteFlags::X)
    }
}

/// One Sv39 page-table page contains 512 eight-byte entries.
#[repr(C, align(4096))]
struct PageTablePage {
    entries: [PageTableEntry; SV39_PAGE_TABLE_ENTRY_COUNT],
}

impl PageTablePage {
    fn clear(&mut self) {
        self.entries.fill(PageTableEntry::empty());
    }

    /// Clear the page-table page located at `ppn` through the identity map.
    ///
    /// # Safety
    ///
    /// `ppn` must refer to a valid, live page-table frame that is accessible
    /// through the current address space. No other mutable access to the page
    /// may be active while it is being cleared.
    unsafe fn zero_at(ppn: PhysPageNum) {
        let address = PhysAddr::from(ppn).get();
        let page = address as *mut PageTablePage;

        unsafe {
            (*page).clear();
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PageTableError {
    OutOfMemory,
    InvalidToken(usize),
    AlreadyMapped(VirtPageNum),
    NotMapped(VirtPageNum),
    EncounteredLeaf(VirtPageNum),
    UnsupportedPageSize(VirtPageNum),
    InvalidRange,
    InvalidFlags(PteFlags),
    CorruptPageTable(VirtPageNum),
    ReadOnlyView,

    Address(AddressError),
    FrameAllocator(FrameAllocatorError),
}

impl From<AddressError> for PageTableError {
    fn from(error: AddressError) -> Self {
        Self::Address(error)
    }
}

impl From<FrameAllocatorError> for PageTableError {
    fn from(error: FrameAllocatorError) -> Self {
        Self::FrameAllocator(error)
    }
}

/// A page table owns the frames used by its root and intermediate nodes.
/// Leaf/data frames passed to `map` remain owned by the caller.
///
/// The implementation currently assumes physical memory is identity mapped:
/// a physical address can be used as a kernel virtual address while walking a
/// table. That is true before paging is enabled and remains true if the kernel
/// installs an identity mapping for its page-table frames.
pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
    owns_frames: bool,
}

impl PageTable {
    /// Allocate and clear a fresh three-level Sv39 root table.
    pub fn new() -> Result<Self, PageTableError> {
        let root = frame_allocator::alloc_frame()?.ok_or(PageTableError::OutOfMemory)?;
        let root_ppn = root.ppn();
        unsafe { PageTablePage::zero_at(root_ppn) };

        Ok(Self {
            root_ppn,
            frames: vec![root],
            owns_frames: true,
        })
    }
}
