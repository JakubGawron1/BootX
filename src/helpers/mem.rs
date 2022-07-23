//! Copyright (c) ChefKiss Inc 2021-2022.
//! This project is licensed by the Creative Commons Attribution-NoCommercial-NoDerivatives licence.

use alloc::vec::Vec;

use sulfur_dioxide::tags::memory_map::{MemoryData, MemoryEntry};

#[derive(Debug)]
pub struct MemoryManager {
    entries: Vec<(usize, usize)>,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn allocate(&mut self, ent: (usize, usize)) {
        self.entries.push(ent)
    }

    pub fn mem_type_from_desc(
        &self,
        desc: &uefi::table::boot::MemoryDescriptor,
    ) -> Option<MemoryEntry> {
        let mut data = MemoryData {
            base: desc.phys_start.try_into().unwrap(),
            length: (desc.page_count * 0x1000).try_into().unwrap(),
        };

        match desc.ty {
            uefi::table::boot::MemoryType::CONVENTIONAL => Some(MemoryEntry::Usable(data)),
            uefi::table::boot::MemoryType::LOADER_CODE
            | uefi::table::boot::MemoryType::LOADER_DATA => {
                let mut ret = MemoryEntry::BootLoaderReclaimable(data);

                for (base, size) in &self.entries {
                    let top = data.base + data.length;
                    if data.base < (base + size) {
                        if top > (base + size) {
                            data.length -= size;
                            data.base += size;
                            ret = MemoryEntry::BootLoaderReclaimable(data);
                        } else {
                            ret = MemoryEntry::KernelOrModule(data);
                        }

                        break;
                    }
                }
                Some(ret)
            }
            uefi::table::boot::MemoryType::ACPI_RECLAIM => Some(MemoryEntry::ACPIReclaimable(data)),
            _ => None,
        }
    }
}
