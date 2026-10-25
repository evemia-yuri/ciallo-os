use bitflags::bitflags;

use crate::mm::address::{AddressError, PhysPageNum};

const PTE_PPN_SHIFT: usize = 0x0a;
const PTE_PPN_BITS: usize = 0x2c;
const PTE_PPN_MASK: usize = (1_usize << PTE_PPN_BITS) - 1;
const PTE_FLAGS_MASK: usize = (1_usize << 8) - 1;

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
            bits: (ppn.get() << PTE_PPN_SHIFT) | flags.bits(),
        }
    }

    pub const fn bits(self) -> usize {
        self.bits
    }

    pub fn ppn(self) -> Result<PhysPageNum, AddressError> {
        let raw_ppn = (self.bits >> PTE_PPN_SHIFT) & PTE_PPN_MASK;
        raw_ppn.try_into()
    }

    pub fn flags(self) -> PteFlags {
        PteFlags::from_bits_retain(self.bits & PTE_FLAGS_MASK)
    }

    pub fn is_valid(self) -> bool {
        self.flags().contains(PteFlags::V)
    }

    pub fn is_leaf(self) -> bool {
        self.flags()
            .intersects(PteFlags::R | PteFlags::W | PteFlags::X)
    }
}
