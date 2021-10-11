use alloc::boxed::Box;
use bit::BitIndex;
use modular_bitfield::prelude::*;

pub const PHYS_VIRT_OFFSET: u64 = 0xFFFF800000000000;
pub const KERNEL_VIRT_OFFSET: u64 = 0xFFFFFFFF80000000;

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
}

impl Default for PageTable {
    fn default() -> Self {
        Self {
            entries: [PageTableEntry::default(); 512],
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PageTableEntryFlags {
    pub present: bool,
    pub writable: bool,
    pub user: bool,
    pub wt: bool,
    pub no_cache: bool,
    pub huge: bool,
    pub global: bool,
    pub no_execute: bool,
}

#[bitfield(bits = 64)]
#[repr(C, u64)]
#[derive(Debug, Default, Clone, Copy)]
pub struct PageTableEntry {
    pub present: bool,
    pub writable: bool,
    pub user: bool,
    pub wt: bool,
    pub no_cache: bool,
    #[skip(setters)]
    pub accessed: bool,
    #[skip(setters)]
    pub dirty: bool,
    pub huge: bool,
    pub global: bool,
    pub available_to_os: B3,
    pub address: B40,
    pub available_to_os2: B11,
    pub no_execute: bool,
}

#[derive(Debug)]
struct PageTableOffsets {
    pub pml4: usize,
    pub pml3: usize,
    pub pml2: usize,
    pub pml1: usize,
}

impl PageTableOffsets {
    pub fn new(virtual_address: u64) -> Self {
        Self {
            pml4: virtual_address.bit_range(39..48).try_into().unwrap(),
            pml3: virtual_address.bit_range(30..39).try_into().unwrap(),
            pml2: virtual_address.bit_range(21..30).try_into().unwrap(),
            pml1: virtual_address.bit_range(12..21).try_into().unwrap(),
        }
    }
}

unsafe fn get_or_alloc_entry(previous_level: *mut PageTable, offset: usize, flags: PageTableEntryFlags) -> *mut PageTable {
    let entry = &mut (*previous_level).entries[offset];
    if entry.present() == false {
        let table = Box::new(PageTable::default());
        entry.set_address((&*table as *const PageTable as u64) >> 12);
        Box::leak(table);

        entry.set_present(flags.present);
        entry.set_writable(flags.writable);
        entry.set_user(flags.user);
        entry.set_wt(flags.wt);
        entry.set_no_cache(flags.no_cache);
        entry.set_huge(flags.huge);
        entry.set_global(flags.global);
        entry.set_no_execute(flags.no_execute);
    }

    (entry.address() << 12) as *mut PageTable
}

unsafe fn map_huge_pages(pml4: *mut PageTable, virt: u64, phys: u64, count: u64) {
    let flags = PageTableEntryFlags {
        present: true,
        writable: true,
        user: true,
        ..Default::default()
    };

    for i in 0..count {
        let physical_address = phys + 0x200000 * i;
        let virtual_address = virt + 0x200000 * i;
        let offs = PageTableOffsets::new(virtual_address);
        let pml3 = get_or_alloc_entry(pml4, offs.pml4, flags);
        let pml2 = get_or_alloc_entry(pml3, offs.pml3, flags);
        (*pml2).entries[offs.pml2 as usize] = PageTableEntry::new().with_present(true).with_writable(true).with_huge(true).with_address(physical_address >> 12);
    }
}

pub unsafe fn map_higher_half(pml4: *mut PageTable) {
    map_huge_pages(pml4, PHYS_VIRT_OFFSET + 0x200000, 0, 2047);
    map_huge_pages(pml4, KERNEL_VIRT_OFFSET, 0, 1024);
}
