/*
 * Copyright (c) VisualDevelopment 2021-2022.
 * This project is licensed by the Creative Commons Attribution-NoCommercial-NoDerivatives licence.
 */

pub mod fb;
pub mod file;
pub mod mem;
pub mod parse_elf;
pub mod setup;

use crate::alloc::boxed::Box;

#[repr(transparent)]
#[derive(Debug)]
pub struct Pml4(amd64::paging::PageTable);

impl amd64::paging::pml4::Pml4 for Pml4 {
    const VIRT_OFF: usize = 0;

    fn get_entry(&mut self, offset: usize) -> &mut amd64::paging::PageTableEntry {
        &mut self.0.entries[offset]
    }

    fn alloc_entry() -> usize {
        Box::leak(Box::new(amd64::paging::PageTable::new())) as *mut _ as usize
    }
}
