pub const KERNEL_HEAP_SIZE: usize = 4 * 1024 * 1024;

pub const PHYS_MEMORY_END: usize = 0x8080_0000;

/// Sv39 uses 4 KiB base pages: 2^12 bytes per page.
pub const SV39_PAGE_OFFSET_BITS: usize = 12;
pub const SV39_PAGE_SIZE: usize = 1 << SV39_PAGE_OFFSET_BITS;

/// Maximum physical-address width assumed by this RISC-V platform reference.
/// Change this if the target platform exposes a different physical address width.
pub const SV39_PA_BITS: usize = 56;
pub const SV39_PPN_BITS: usize = SV39_PA_BITS - SV39_PAGE_OFFSET_BITS;
/// Exclusive upper bound of representable physical byte addresses.
pub const SV39_PA_LIMIT: usize = 1 << SV39_PA_BITS;
/// Number of base pages in the representable physical address range.
pub const SV39_PPN_LIMIT: usize = 1 << SV39_PPN_BITS;

/// Sv39 virtual addresses use bits 38:0; bits 63:39 must sign-extend bit 38.
pub const SV39_VA_BITS: usize = 39;
/// VPN is VA[38:12], leaving the low 12 bits for the page offset.
pub const SV39_VPN_BITS: usize = SV39_VA_BITS - SV39_PAGE_OFFSET_BITS;
pub const SV39_VA_LIMIT: usize = 1 << SV39_VA_BITS;
pub const SV39_VPN_LIMIT: usize = 1 << SV39_VPN_BITS;
/// VPN[26] selects the high canonical half; VPN values below this are low-half.
pub const SV39_VPN_HALF_COUNT: usize = 1 << (SV39_VPN_BITS - 1);

/// Each page-table level has 512 entries because its index is 9 bits wide.
pub const SV39_PTE_COUNT: usize = 512;
/// Sv39 page tables have levels 2, 1, and 0.
pub const SV39_PT_LEVELS: usize = 3;

/// In a 64-bit Sv39 PTE, the 44-bit PPN field starts at bit 10.
pub const SV39_PTE_PPN_SHIFT: usize = 10;
pub const SV39_PTE_PPN_BITS: usize = 44;
/// Mask selecting the 44 PPN bits after shifting them down to bit zero.
pub const SV39_PTE_PPN_MASK: usize = (1 << SV39_PTE_PPN_BITS) - 1;
/// The low 8 PTE bits are V/R/W/X/U/G/A/D; higher reserved bits are excluded.
pub const SV39_PTE_FLAGS_MASK: usize = (1 << 8) - 1;

/// SATP mode 8 selects Sv39, stored in SATP bits 63:60.
pub const SATP_MODE_SV39: usize = 8;
/// Number of low bits below SATP.MODE (ASID occupies bits 59:44).
pub const SATP_MODE_SHIFT: usize = 60;
