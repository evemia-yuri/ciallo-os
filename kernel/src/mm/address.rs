const PAGE_SIZE_BITS: usize = 0x0c;
const PAGE_SIZE: usize = 1 << PAGE_SIZE_BITS;

const PHYS_ADDR_BITS: usize = 0x38;
const PHYS_ADDR_LIMIT: usize = 1_usize << PHYS_ADDR_BITS;
const PHYS_PAGE_COUNT: usize = PHYS_ADDR_LIMIT / PAGE_SIZE;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AddressError {
    PhysicalAddressOutOfRange(usize),
    PhysicalPageOutOfRange(usize),
    NotPageAligned(PhysAddr),
}

/// A byte address in physical memory.
///
/// The field is private so callers cannot create an invalid value by writing
/// `PhysAddr(raw)` directly. Use `PhysAddr::try_from(raw)` instead.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct PhysAddr(usize);

impl PhysAddr {
    pub const fn get(self) -> usize {
        self.0
    }

    pub const fn page_offset(self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }

    pub const fn is_aligned(self) -> bool {
        self.page_offset() == 0
    }

    /// Round down to the page containing this address.
    pub const fn floor(self) -> PhysPageNum {
        PhysPageNum(self.0 >> PAGE_SIZE_BITS)
    }

    /// Round up to the first page after this address.
    ///
    /// The result is useful as an exclusive range endpoint. It is fallible
    /// because the maximum address can round up to one-past the last page,
    /// which is not itself a valid `PhysPageNum`.
    pub fn ceil(&self) -> Result<PhysPageNum, AddressError> {
        PhysPageNum::try_from((self.0 + PAGE_SIZE - 1) >> PAGE_SIZE_BITS)
    }
}

impl TryFrom<usize> for PhysAddr {
    type Error = AddressError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value < PHYS_ADDR_LIMIT {
            Ok(PhysAddr(value))
        } else {
            Err(AddressError::PhysicalAddressOutOfRange(value))
        }
    }
}

impl From<PhysAddr> for usize {
    fn from(pa: PhysAddr) -> Self {
        pa.0
    }
}

/// A physical page number.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct PhysPageNum(usize);

impl PhysPageNum {
    pub const fn get(self) -> usize {
        self.0
    }

    /// Add a number of pages without leaving the representable PPN range.
    pub fn checked_add(self, pages: usize) -> Option<Self> {
        let value = self.0.checked_add(pages)?;
        Self::try_from(value).ok()
    }
}

impl TryFrom<usize> for PhysPageNum {
    type Error = AddressError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value < PHYS_PAGE_COUNT {
            Ok(PhysPageNum(value))
        } else {
            Err(AddressError::PhysicalPageOutOfRange(value))
        }
    }
}

impl From<PhysPageNum> for usize {
    fn from(ppn: PhysPageNum) -> Self {
        ppn.0
    }
}

impl TryFrom<PhysAddr> for PhysPageNum {
    type Error = AddressError;

    fn try_from(pa: PhysAddr) -> Result<Self, Self::Error> {
        if pa.is_aligned() {
            Ok(pa.floor())
        } else {
            Err(AddressError::NotPageAligned(pa))
        }
    }
}

impl From<PhysPageNum> for PhysAddr {
    fn from(ppn: PhysPageNum) -> Self {
        // Every constructible PhysPageNum is in range, so this is lossless and
        // cannot fail. This is the kind of conversion that fits `From`.
        Self(ppn.0 << PAGE_SIZE_BITS)
    }
}
