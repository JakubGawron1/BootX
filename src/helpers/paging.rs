use ::alloc::*;
use bit::BitIndex;
use modular_bitfield::prelude::*;

pub const PHYS_VIRT_OFFSET: u64 = 0xFFFF800000000000;
pub const KERNEL_VIRT_OFFSET: u64 = 0xFFFFFFFF80000000;

#[derive(Debug)]
#[repr(C, packed)]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
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

#[bitfield(bits = 64, filled = true)]
#[repr(u64)]
#[derive(Debug)]
pub struct PageTableEntry {
    pub present: B1,
    pub writable: B1,
    pub user: B1,
    pub wt: B1,
    pub no_cache: B1,
    #[skip(setters)]
    pub accessed: B1,
    #[skip(setters)]
    pub dirty: B1,
    pub huge: B1,
    pub global: B1,
    pub available_to_os: B3,
    pub address: B40,
    pub available_to_os2: B11,
    pub no_execute: B1,
}

#[derive(Debug)]
struct PageTableOffsets {
    pub pml4: u16,
    pub pml3: u16,
    pub pml2: u16,
    pub pml1: u16,
}

impl PageTableOffsets {
    pub fn new(virtual_address: u64) -> PageTableOffsets {
        PageTableOffsets {
            pml4: virtual_address.bit_range(39..48) as u16,
            pml3: virtual_address.bit_range(30..39) as u16,
            pml2: virtual_address.bit_range(21..30) as u16,
            pml1: virtual_address.bit_range(12..21) as u16,
        }
    }
}

fn get_or_alloc_entry(previous_level: *mut PageTable, offset: u16, flags: PageTableEntryFlags) -> *mut PageTable {
    let entry = unsafe { &mut (*previous_level).entries[offset as usize] };
    if entry.present() == 0 {
        unsafe {
            let layout = core::alloc::Layout::new::<PageTable>();
            let new_ptr = alloc::alloc_zeroed(layout);
            entry.set_address(
                match core::ptr::NonNull::new(new_ptr as *mut PageTable) {
                    Some(p) => p,
                    None => alloc::handle_alloc_error(layout),
                }
                .as_ptr() as u64
                    >> 12,
            );
        }

        entry.set_present(flags.present as u8);
        entry.set_writable(flags.writable as u8);
        entry.set_user(flags.user as u8);
        entry.set_wt(flags.wt as u8);
        entry.set_no_cache(flags.no_cache as u8);
        entry.set_huge(flags.huge as u8);
        entry.set_global(flags.global as u8);
        entry.set_no_execute(flags.no_execute as u8);
    }

    (entry.address() << 12) as *mut PageTable
}

fn map_huge_pages(pml4: *mut PageTable, virt: u64, phys: u64, count: u64) {
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
        unsafe {
            (*pml2).entries[offs.pml2 as usize] = PageTableEntry::new().with_present(1).with_writable(1).with_huge(1).with_address(physical_address >> 12);
        }
    }
}

pub fn map_higher_half(pml4: *mut PageTable) {
    map_huge_pages(pml4, PHYS_VIRT_OFFSET, 0, 512 * 4);
    map_huge_pages(pml4, KERNEL_VIRT_OFFSET, 0, 64);
}
