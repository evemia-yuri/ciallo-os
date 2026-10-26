use alloc::vec;
use alloc::vec::Vec;
use core::ops::Range;
use core::ptr::addr_of_mut;

use bitflags::bitflags;
use riscv::asm;
use riscv::register::satp::{self, Mode::Sv39};

use crate::config::{
    SATP_MODE_SHIFT, SATP_MODE_SV39, SV39_PAGE_SIZE, SV39_PT_LEVELS, SV39_PTE_COUNT,
    SV39_PTE_FLAGS_MASK, SV39_PTE_PPN_MASK, SV39_PTE_PPN_SHIFT, SV39_VPN_HALF_COUNT,
};
use crate::mm::address::{AddressError, PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use crate::mm::frame_allocator::{self, FrameAllocatorError, FrameTracker};

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
    entries: [PageTableEntry; SV39_PTE_COUNT],
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

    /// Get a pointer to one entry in the page-table page at `ppn`.
    ///
    /// # Safety
    ///
    /// `ppn` must refer to a valid, live page-table frame accessible through
    /// the current address space, and `index` must be less than
    /// [`SV39_PTE_COUNT`]. The caller must ensure accesses through the
    /// returned pointer do not conflict with other accesses.
    unsafe fn entry_ptr_at(ppn: PhysPageNum, index: usize) -> *mut PageTableEntry {
        let address = PhysAddr::from(ppn).get();
        let page = address as *mut PageTablePage;
        unsafe { addr_of_mut!((*page).entries[index]) }
    }
}

const _: [(); SV39_PAGE_SIZE] = [(); size_of::<PageTablePage>()];

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

    pub const fn root_ppn(&self) -> PhysPageNum {
        self.root_ppn
    }

    /// Return the value that should be written to `satp` for this table.
    pub const fn token(&self) -> usize {
        (SATP_MODE_SV39 << SATP_MODE_SHIFT) | self.root_ppn.get()
    }

    /// Construct a non-owning view of the root selected by a SATP token.
    ///
    /// The returned value must not outlive the page-table memory represented by
    /// the token. It does not reclaim the root or intermediate frames when
    /// dropped, because it did not allocate them.
    /// # Safety
    ///
    /// `token` must name a live, valid Sv39 page table. Its page-table frames
    /// must be accessible through the current kernel mapping and must not be
    /// concurrently modified while this view is being used.
    pub unsafe fn from_token(token: usize) -> Result<Self, PageTableError> {
        if token >> SATP_MODE_SHIFT != SATP_MODE_SV39 {
            return Err(PageTableError::InvalidToken(token));
        }

        let root_ppn = (token & SV39_PTE_PPN_MASK).try_into()?;
        Ok(Self {
            root_ppn,
            frames: Vec::new(),
            owns_frames: false,
        })
    }

    /// Activate this table for the current hart.
    ///
    /// # Safety
    ///
    /// The table must keep identity mappings for the currently executing
    /// kernel code, stack, trap entry, and any data touched immediately after
    /// the SATP write. The table must remain alive while it is active.
    pub unsafe fn activate(&self) {
        unsafe {
            satp::set(Sv39, 0, self.root_ppn.get());
        }
        asm::sfence_vma_all();
    }

    /// Return the leaf PTE for a virtual page, if it is mapped.
    pub fn translate(&self, vpn: VirtPageNum) -> Result<Option<PageTableEntry>, PageTableError> {
        self.find_pte(vpn)
    }

    /// Translate a virtual byte address to a physical byte address.
    pub fn translate_va(&self, va: VirtAddr) -> Result<Option<PhysAddr>, PageTableError> {
        let Some(pte) = self.translate(va.floor())? else {
            return Ok(None);
        };

        let ppn = pte.ppn()?;
        let physical = PhysAddr::from(ppn)
            .get()
            .checked_add(va.page_offset())
            .ok_or(AddressError::PhysicalAddressOutOfRange(usize::MAX))?;
        Ok(Some(physical.try_into()?))
    }

    /// Map one 4 KiB virtual page to one physical page.
    pub fn map(
        &mut self,
        vpn: VirtPageNum,
        ppn: PhysPageNum,
        flags: PteFlags,
    ) -> Result<(), PageTableError> {
        self.ensure_owned()?;
        self.map_inner(vpn, ppn, flags)?;
        self.flush_tlb();
        Ok(())
    }

    /// Map a contiguous virtual-page range to a contiguous physical-page
    /// range. A range cannot straddle Sv39's gap between its low and high
    /// canonical halves. If one mapping fails, mappings created by this call
    /// are rolled back. Any intermediate page-table pages allocated during
    /// the attempt remain owned by this table for reuse.
    pub fn map_range(
        &mut self,
        vpn_range: Range<VirtPageNum>,
        ppn_start: PhysPageNum,
        flags: PteFlags,
    ) -> Result<(), PageTableError> {
        self.ensure_owned()?;
        let page_count = vpn_range
            .end
            .get()
            .checked_sub(vpn_range.start.get())
            .ok_or(PageTableError::InvalidRange)?;
        if vpn_range.start.get() < SV39_VPN_HALF_COUNT && vpn_range.end.get() > SV39_VPN_HALF_COUNT
        {
            return Err(PageTableError::InvalidRange);
        }
        if page_count != 0 && ppn_start.checked_add(page_count - 1).is_none() {
            return Err(PageTableError::OutOfMemory);
        }

        let mut mapped = 0;
        while mapped < page_count {
            let vpn = VirtPageNum::try_from(vpn_range.start.get() + mapped)?;
            let ppn = ppn_start
                .checked_add(mapped)
                .ok_or(PageTableError::OutOfMemory)?;
            if let Err(error) = self.map_inner(vpn, ppn, flags) {
                for rollback in 0..mapped {
                    let rollback_vpn = VirtPageNum::try_from(vpn_range.start.get() + rollback)
                        .expect("rollback VPN was previously validated");
                    let _ = self.unmap_inner(rollback_vpn);
                }
                self.flush_tlb();
                return Err(error);
            }
            mapped += 1;
        }

        if mapped != 0 {
            self.flush_tlb();
        }

        Ok(())
    }

    /// Remove one 4 KiB mapping. Intermediate page-table pages are retained
    /// so that a later mapping can reuse them without another allocation.
    pub fn unmap(&mut self, vpn: VirtPageNum) -> Result<(), PageTableError> {
        self.ensure_owned()?;
        self.unmap_inner(vpn)?;
        self.flush_tlb();
        Ok(())
    }

    fn map_inner(
        &mut self,
        vpn: VirtPageNum,
        ppn: PhysPageNum,
        flags: PteFlags,
    ) -> Result<(), PageTableError> {
        let leaf_flags = flags & (PteFlags::R | PteFlags::W | PteFlags::X);
        if !leaf_flags.intersects(PteFlags::R | PteFlags::W | PteFlags::X)
            || (flags.contains(PteFlags::W) && !flags.contains(PteFlags::R))
        {
            return Err(PageTableError::InvalidFlags(flags));
        }

        let pte_ptr = self.find_ptr_create(vpn)?;
        let old_pte = unsafe { pte_ptr.read_volatile() };
        if old_pte.is_valid() {
            return Err(PageTableError::AlreadyMapped(vpn));
        }

        // Set A/D eagerly so this also works on implementations that raise
        // page faults instead of updating these bits in hardware.
        let mut installed_flags = flags | PteFlags::V | PteFlags::A;
        if flags.contains(PteFlags::W) {
            installed_flags |= PteFlags::D;
        }
        unsafe { pte_ptr.write_volatile(PageTableEntry::new(ppn, installed_flags)) };

        Ok(())
    }

    fn unmap_inner(&mut self, vpn: VirtPageNum) -> Result<(), PageTableError> {
        let pte_ptr = self
            .find_pte_existing(vpn)?
            .ok_or(PageTableError::NotMapped(vpn))?;
        let pte = unsafe { pte_ptr.read_volatile() };
        if !pte.is_leaf() {
            return Err(PageTableError::CorruptPageTable(vpn));
        }

        unsafe {
            pte_ptr.write_volatile(PageTableEntry::empty());
        }
        Ok(())
    }

    fn find_ptr_create(&mut self, vpn: VirtPageNum) -> Result<*mut PageTableEntry, PageTableError> {
        let indexes = vpn.indexes();
        let mut ppn = self.root_ppn;
        for index in indexes.iter().take(SV39_PT_LEVELS - 1) {
            let next_ppn = {
                let pte_ptr = unsafe { PageTablePage::entry_ptr_at(ppn, *index) };
                let pte = unsafe { pte_ptr.read_volatile() };
                if pte.is_valid() {
                    if pte.is_leaf() {
                        // An intermediate node is acting as a leaf node,
                        // indicating that a huge page has been encountered;
                        // however, the current system does not support huge pages.
                        return Err(PageTableError::EncounteredLeaf(vpn));
                    }
                    pte.ppn()?
                } else {
                    let frame =
                        frame_allocator::alloc_frame()?.ok_or(PageTableError::OutOfMemory)?;
                    let child_ppn = frame.ppn();
                    unsafe {
                        PageTablePage::zero_at(child_ppn);
                    }

                    unsafe {
                        pte_ptr.write_volatile(PageTableEntry::new(child_ppn, PteFlags::V));
                    }
                    self.frames.push(frame);
                    child_ppn
                }
            };

            ppn = next_ppn;
        }

        Ok(unsafe { PageTablePage::entry_ptr_at(ppn, indexes[SV39_PT_LEVELS - 1]) })
    }

    fn find_pte(&self, vpn: VirtPageNum) -> Result<Option<PageTableEntry>, PageTableError> {
        let indexes = vpn.indexes();
        let mut ppn = self.root_ppn;
        for (level, index) in indexes.iter().enumerate() {
            let pte = unsafe { PageTablePage::entry_ptr_at(ppn, *index).read_volatile() };
            if !pte.is_valid() {
                return Ok(None);
            }

            if pte.is_leaf() {
                if level + 1 == SV39_PT_LEVELS {
                    return Ok(Some(pte));
                }

                // Reaching this point also implies that a "huge page"
                // has been encountered, which the system is unable to handle.
                return Err(PageTableError::UnsupportedPageSize(vpn));
            }

            // Reaching the final level without arriving at a leaf node
            // indicates that the page table has been corrupted.
            if level + 1 == SV39_PT_LEVELS {
                return Err(PageTableError::CorruptPageTable(vpn));
            }
            ppn = pte.ppn()?
        }

        Ok(None)
    }

    fn find_pte_existing(
        &self,
        vpn: VirtPageNum,
    ) -> Result<Option<*mut PageTableEntry>, PageTableError> {
        let indexes = vpn.indexes();
        let mut ppn = self.root_ppn;
        for (level, index) in indexes.iter().enumerate() {
            let pte_ptr = unsafe { PageTablePage::entry_ptr_at(ppn, *index) };
            let pte = unsafe { pte_ptr.read_volatile() };
            if !pte.is_valid() {
                return Ok(None);
            }

            if pte.is_leaf() {
                if level + 1 == SV39_PT_LEVELS {
                    return Ok(Some(pte_ptr));
                }

                return Err(PageTableError::UnsupportedPageSize(vpn));
            }

            if level + 1 == SV39_PT_LEVELS {
                return Err(PageTableError::CorruptPageTable(vpn));
            }
            ppn = pte.ppn()?
        }

        Ok(None)
    }

    fn flush_tlb(&self) {
        asm::sfence_vma_all();
    }

    fn ensure_owned(&self) -> Result<(), PageTableError> {
        if self.owns_frames {
            Ok(())
        } else {
            Err(PageTableError::ReadOnlyView)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::SV39_PAGE_SIZE;
    use crate::mm::address::{AddressError, PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
    use crate::mm::page_table::{
        PageTable, PageTableEntry, PageTableError, PteFlags, SV39_VPN_HALF_COUNT,
    };

    #[test_case]
    fn pte_encodes_ppn_and_flags() {
        let ppn = PhysPageNum::try_from(0x12345).unwrap();
        let pte = PageTableEntry::new(ppn, PteFlags::V | PteFlags::R);

        assert_eq!(pte.ppn().unwrap(), ppn);
        assert_eq!(pte.flags(), PteFlags::V | PteFlags::R);
        assert!(pte.is_valid());
        assert!(pte.is_leaf());
    }

    #[test_case]
    fn vpn_indexes_are_in_walk_order() {
        // Example VPN whose level-2/1/0 indexes are respectively 2, 3, and 4.
        let vpn = VirtPageNum::try_from((2 << 18) | (3 << 9) | 4).unwrap();
        assert_eq!(vpn.indexes(), [2, 3, 4]);
    }

    #[test_case]
    fn virtual_address_rounds_and_preserves_offset() {
        let va = VirtAddr::try_from(0x1234_5678).unwrap();
        assert_eq!(va.page_offset(), 0x678);
        assert_eq!(
            VirtAddr::from(va.floor()).get() + va.page_offset(),
            va.get()
        );
    }

    #[test_case]
    fn virtual_address_try_into_vpn_requires_page_alignment() {
        let aligned = VirtAddr::try_from(0x1234_5000).unwrap();
        let vpn = VirtPageNum::try_from(aligned).unwrap();
        assert_eq!(VirtAddr::from(vpn), aligned);

        let unaligned = VirtAddr::try_from(0x1234_5678).unwrap();
        assert_eq!(
            VirtPageNum::try_from(unaligned),
            Err(AddressError::VirtualAddressNotPageAligned(unaligned))
        );
        assert_eq!(
            unaligned.floor(),
            VirtAddr::try_from(0x1234_5000).unwrap().floor()
        );
    }

    #[test_case]
    fn virtual_address_ceil_rounds_up_and_checks_sv39_limit() {
        let aligned = VirtAddr::try_from(0x1234_5000).unwrap();
        assert_eq!(aligned.ceil().unwrap(), aligned.floor());

        let unaligned = VirtAddr::try_from(0x1234_5678).unwrap();
        assert_eq!(VirtAddr::from(unaligned.ceil().unwrap()).get(), 0x1234_6000);

        // The final canonical Sv39 byte address cannot round up to another VPN.
        let high_end = VirtAddr::try_from(usize::MAX).unwrap();
        // 2^27 is one past the largest normalized 27-bit VPN.
        assert_eq!(
            high_end.ceil(),
            Err(AddressError::VirtualPageOutOfRange(1 << 27))
        );
    }

    #[test_case]
    fn sv39_accepts_only_canonical_virtual_addresses() {
        assert!(VirtAddr::try_from(0x0000_003f_ffff_ffff).is_ok());
        assert!(VirtAddr::try_from(0x0000_0040_0000_0000).is_err());
        assert!(VirtAddr::try_from(0xffff_ffc0_0000_0000).is_ok());
        assert!(VirtAddr::try_from(0xffff_ffbf_ffff_ffff).is_err());

        let high = VirtPageNum::try_from((1 << 26) + 0x1234).unwrap();
        let va = VirtAddr::from(high);
        assert_eq!(VirtAddr::try_from(va.get()).unwrap().floor(), high);
    }

    #[test_case]
    fn page_table_maps_translates_and_unmaps() {
        let mut page_table = PageTable::new().unwrap();
        let va = VirtAddr::try_from(0x4000_1234).unwrap();
        let vpn = va.floor();
        let ppn = PhysPageNum::try_from(0x8020_0000 / SV39_PAGE_SIZE).unwrap();

        assert_eq!(page_table.translate(vpn).unwrap(), None);
        page_table.map(vpn, ppn, PteFlags::R | PteFlags::W).unwrap();
        assert_eq!(
            page_table.translate_va(va).unwrap(),
            Some(
                PhysAddr::from(ppn)
                    .get()
                    .checked_add(va.page_offset())
                    .unwrap()
                    .try_into()
                    .unwrap()
            )
        );
        assert_eq!(
            page_table.map(vpn, ppn, PteFlags::R),
            Err(PageTableError::AlreadyMapped(vpn))
        );

        page_table.unmap(vpn).unwrap();
        assert_eq!(page_table.translate(vpn).unwrap(), None);
        assert_eq!(page_table.unmap(vpn), Err(PageTableError::NotMapped(vpn)));
    }

    #[test_case]
    fn map_range_failure_rolls_back_earlier_mappings() {
        let mut page_table = PageTable::new().unwrap();
        let vpn_start = VirtPageNum::try_from(0x80000).unwrap();
        let vpn_middle = vpn_start.checked_add(1).unwrap();
        let ppn = PhysPageNum::try_from(0x8020_0000 / SV39_PAGE_SIZE).unwrap();

        page_table
            .map(vpn_middle, ppn.checked_add(1).unwrap(), PteFlags::R)
            .unwrap();
        assert_eq!(
            page_table.map_range(
                vpn_start..vpn_start.checked_add(3).unwrap(),
                ppn,
                PteFlags::R
            ),
            Err(PageTableError::AlreadyMapped(vpn_middle))
        );
        assert_eq!(page_table.translate(vpn_start).unwrap(), None);
        assert_eq!(
            page_table
                .translate(vpn_middle)
                .unwrap()
                .unwrap()
                .ppn()
                .unwrap(),
            ppn.checked_add(1).unwrap()
        );
        assert_eq!(
            page_table
                .translate(vpn_start.checked_add(2).unwrap())
                .unwrap(),
            None
        );
    }

    #[test_case]
    fn map_range_rejects_sv39_canonical_address_gap() {
        let mut page_table = PageTable::new().unwrap();
        let start = VirtPageNum::try_from(SV39_VPN_HALF_COUNT - 1).unwrap();
        let end = VirtPageNum::try_from(SV39_VPN_HALF_COUNT + 1).unwrap();
        let ppn = PhysPageNum::try_from(0x8020_0000 / SV39_PAGE_SIZE).unwrap();

        assert_eq!(
            page_table.map_range(start..end, ppn, PteFlags::R),
            Err(PageTableError::InvalidRange)
        );
    }

    #[test_case]
    fn map_rejects_invalid_leaf_permissions() {
        let mut page_table = PageTable::new().unwrap();
        let vpn = VirtPageNum::try_from(0x90000).unwrap();
        let ppn = PhysPageNum::try_from(0x8020_0000 / SV39_PAGE_SIZE).unwrap();

        assert_eq!(
            page_table.map(vpn, ppn, PteFlags::W),
            Err(PageTableError::InvalidFlags(PteFlags::W))
        );
        assert_eq!(
            page_table.map(vpn, ppn, PteFlags::V),
            Err(PageTableError::InvalidFlags(PteFlags::V))
        );
    }
}
