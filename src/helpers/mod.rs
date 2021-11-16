/*
 * Copyright (c) VisualDevelopment 2021-2021.
 * This project is licensed by the Creative Commons Attribution-NoCommercial-NoDerivatives licence.
 */

pub mod kaboom;
pub mod load_file;
pub mod parse_elf;
pub mod setup;

use crate::alloc::boxed::Box;

amd64::impl_pml4!(
    Box::leak(Box::new(amd64::paging::PageTable::new())) as *mut _ as usize,
    0usize
);
