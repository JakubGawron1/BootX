use alloc::vec::Vec;

use kaboom::tags::memory_map::{MemoryData, MemoryEntry};

#[derive(Debug)]
pub struct MemoryManager {
    entries: Vec<usize>,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn allocate(&mut self, ent: (usize, usize)) {
        self.entries.push(ent.0 + ent.1)
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
                let mut ret = Some(MemoryEntry::BootLoaderReclaimable(data));

                for ent in &self.entries {
                    let top = data.base + data.length;
                    if data.base < *ent {
                        if top > *ent {
                            data.length -= ent - data.base;
                            data.base += ent;
                            ret = Some(MemoryEntry::BootLoaderReclaimable(data));
                        } else {
                            ret = Some(MemoryEntry::KernelOrModule(data));
                        }

                        break;
                    }
                }
                ret
            }
            uefi::table::boot::MemoryType::ACPI_RECLAIM => Some(MemoryEntry::ACPIReclaimable(data)),
            _ => None,
        }
    }
}
