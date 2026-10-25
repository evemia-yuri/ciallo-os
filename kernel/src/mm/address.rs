/// Sv39 uses 4 KiB base pages: 2^12 bytes per page.
const SV39_PAGE_OFFSET_BITS: usize = 12;
const SV39_PAGE_SIZE: usize = 1 << SV39_PAGE_OFFSET_BITS;

/// Maximum physical-address width assumed by this RISC-V platform reference.
/// Change this if the target platform exposes a different physical address width.
const SV39_PA_BITS: usize = 56;
const SV39_PPN_BITS: usize = SV39_PA_BITS - SV39_PAGE_OFFSET_BITS;
/// Exclusive upper bound of representable physical byte addresses.
const SV39_PA_LIMIT: usize = 1 << SV39_PA_BITS;
/// Number of base pages in the representable physical address range.
const SV39_PPN_LIMIT: usize = 1 << SV39_PPN_BITS;

/// Sv39 virtual addresses use bits 38:0; bits 63:39 must sign-extend bit 38.
const SV39_VA_BITS: usize = 39;
/// VPN is VA[38:12], leaving the low 12 bits for the page offset.
const SV39_VPN_BITS: usize = SV39_VA_BITS - SV39_PAGE_OFFSET_BITS;
const SV39_VA_LIMIT: usize = 1 << SV39_VA_BITS;
const SV39_VPN_LIMIT: usize = 1 << SV39_VPN_BITS;
/// VPN[26] selects the high canonical half; VPN values below this are low-half.
const SV39_HALF_VPN_COUNT: usize = 1 << (SV39_VPN_BITS - 1);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AddressError {
    PhysicalAddressOutOfRange(usize),
    PhysicalPageOutOfRange(usize),
    PhysicalAddressNotPageAligned(PhysAddr),

    NonCanonicalVirtualAddress(usize),
    VirtualPageOutOfRange(usize),
    VirtualAddressNotPageAligned(VirtAddr),
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
        self.0 & (SV39_PAGE_SIZE - 1)
    }

    pub const fn is_aligned(self) -> bool {
        self.page_offset() == 0
    }

    /// Round down to the page containing this address.
    pub const fn floor(self) -> PhysPageNum {
        PhysPageNum(self.0 >> SV39_PAGE_OFFSET_BITS)
    }

    /// Round up to the first page after this address.
    ///
    /// The result is useful as an exclusive range endpoint. It is fallible
    /// because the maximum address can round up to one-past the last page,
    /// which is not itself a valid `PhysPageNum`.
    pub fn ceil(&self) -> Result<PhysPageNum, AddressError> {
        PhysPageNum::try_from((self.0 + SV39_PAGE_SIZE - 1) >> SV39_PAGE_OFFSET_BITS)
    }
}

impl TryFrom<usize> for PhysAddr {
    type Error = AddressError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value < SV39_PA_LIMIT {
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
        if value < SV39_PPN_LIMIT {
            Ok(PhysPageNum(value))
        } else {
            Err(AddressError::PhysicalPageOutOfRange(value))
        }
    }
}

impl From<PhysPageNum> for usize {
    fn from(ppn: PhysPageNum) -> Self {
        ppn.get()
    }
}

impl TryFrom<PhysAddr> for PhysPageNum {
    type Error = AddressError;

    fn try_from(pa: PhysAddr) -> Result<Self, Self::Error> {
        if pa.is_aligned() {
            Ok(pa.floor())
        } else {
            Err(AddressError::PhysicalAddressNotPageAligned(pa))
        }
    }
}

impl From<PhysPageNum> for PhysAddr {
    fn from(ppn: PhysPageNum) -> Self {
        // Every constructible PhysPageNum is in range, so this is lossless and
        // cannot fail. This is the kind of conversion that fits `From`.
        Self(ppn.get() << SV39_PAGE_OFFSET_BITS)
    }
}

/// A canonical Sv39 virtual byte address.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct VirtAddr(usize);

impl VirtAddr {
    pub const fn get(self) -> usize {
        self.0
    }

    pub const fn page_offset(self) -> usize {
        self.0 & (SV39_PAGE_SIZE - 1)
    }

    pub const fn is_aligned(self) -> bool {
        self.page_offset() == 0
    }

    /// Return the normalized 27-bit VPN (VA bits 38:12).
    pub const fn floor(self) -> VirtPageNum {
        VirtPageNum((self.0 >> SV39_PAGE_OFFSET_BITS) & (SV39_VPN_LIMIT - 1))
    }

    /// Round up to the next page boundary, returning its VPN.
    ///
    /// An already aligned address maps to its current VPN. Near the top of the
    /// high canonical Sv39 range, rounding may exceed the representable VPN
    /// range and return `VirtualPageOutOfRange`.
    pub fn ceil(&self) -> Result<VirtPageNum, AddressError> {
        // Unlike physical addresses, we cannot use the standard `(addr + PAGE_SIZE - 1)`
        // math here. Because Sv39 high-half virtual addresses (used by the kernel) are
        // sign-extended, their integer values are extremely close to `usize::MAX`.
        // A direct addition could easily cause an integer overflow panic in Rust.

        // Instead, this method takes a defensive approach: it first calculates the
        // safe `floor` VPN. If the address is unaligned, it safely increments the VPN
        // using `checked_add`, strictly preventing the page from falling into the
        // illegal non-canonical address gap or exceeding the maximum representable VPN.
        let vpn = self.floor();
        if self.is_aligned() {
            Ok(vpn)
        } else {
            vpn.checked_add(1)
                .ok_or(AddressError::VirtualPageOutOfRange(
                    vpn.get().saturating_add(1),
                ))
        }
    }
}

impl TryFrom<usize> for VirtAddr {
    type Error = AddressError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        let upper_bits = value >> (SV39_VA_BITS - 1);
        let sign_extension = usize::MAX >> (SV39_VA_BITS - 1);
        if upper_bits == 0 || upper_bits == sign_extension {
            Ok(Self(value))
        } else {
            Err(AddressError::NonCanonicalVirtualAddress(value))
        }
    }
}

impl From<VirtAddr> for usize {
    fn from(va: VirtAddr) -> Self {
        va.get()
    }
}

/// The normalized 27-bit Sv39 virtual page number (VA bits 38:12).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct VirtPageNum(usize);

impl VirtPageNum {
    pub const fn get(self) -> usize {
        self.0
    }

    /// Return VPN indices in page-table walk order: level 2, 1, then 0.
    pub const fn indexes(self) -> [usize; 3] {
        [
            (self.0 >> 18) & 0x01ff,
            (self.0 >> 9) & 0x01ff,
            self.0 & 0x01ff,
        ]
    }

    pub fn checked_add(self, pages: usize) -> Option<Self> {
        let value = self.0.checked_add(pages)?;
        Self::try_from(value).ok()
    }
}

impl TryFrom<usize> for VirtPageNum {
    type Error = AddressError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value < SV39_VPN_LIMIT {
            Ok(Self(value))
        } else {
            Err(AddressError::VirtualPageOutOfRange(value))
        }
    }
}

impl From<VirtPageNum> for usize {
    fn from(vpn: VirtPageNum) -> Self {
        vpn.get()
    }
}

impl TryFrom<VirtAddr> for VirtPageNum {
    type Error = AddressError;

    /// Convert an aligned virtual byte address to its page number.
    ///
    /// For an arbitrary address that should be rounded down to its containing
    /// page, use `VirtAddr::floor()` explicitly instead.
    fn try_from(va: VirtAddr) -> Result<Self, Self::Error> {
        if va.is_aligned() {
            Ok(va.floor())
        } else {
            Err(AddressError::VirtualAddressNotPageAligned(va))
        }
    }
}

impl From<VirtPageNum> for VirtAddr {
    fn from(vpn: VirtPageNum) -> Self {
        let addr = vpn.get() << SV39_PAGE_OFFSET_BITS;
        if vpn.get() >= SV39_HALF_VPN_COUNT {
            Self(addr | !((1_usize << SV39_VA_BITS) - 1))
        } else {
            Self(addr)
        }
    }
}
