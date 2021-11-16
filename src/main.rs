/*
 * Copyright (c) VisualDevelopment 2021-2021.
 * This project is licensed by the Creative Commons Attribution-NoCommercial-NoDerivatives licence.
 */

#![no_std]
#![no_main]
#![deny(warnings, unused_extern_crates, clippy::cargo, rust_2021_compatibility)]
#![feature(asm)]
#![feature(abi_efiapi)]
#![feature(allocator_api)]
#![feature(core_intrinsics)]
#![feature(asm_const)]

extern crate alloc;

use alloc::{boxed::Box, vec, vec::Vec};

use log::*;
use uefi::{
    prelude::*,
    proto::media::file::{FileAttribute, FileMode},
};

mod helpers;

#[entry]
fn efi_main(image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).expect_success("Failed to initialize utilities");
    helpers::setup::init_output();
    info!("Welcome...");
    helpers::setup::setup_paging();

    let esp = helpers::load_file::open_esp(image);

    let buffer = helpers::load_file::load_file(
        esp,
        "\\System\\fuse.exec",
        FileMode::Read,
        FileAttribute::empty(),
    );

    let (kernel_main, _stack) = helpers::parse_elf::parse_elf(&buffer);

    let mut explosion = Box::new(kaboom::ExplosionResult::new(Default::default()));
    let mut tags = Vec::with_capacity(4);

    tags.push(kaboom::tags::TagType::CommandLine(""));
    tags.push(kaboom::tags::TagType::FrameBuffer(Box::leak(
        helpers::kaboom::fbinfo_from_gop(helpers::setup::get_gop()),
    )));
    tags.push(kaboom::tags::TagType::Acpi(helpers::setup::get_rsdp()));

    info!("Exiting boot services and jumping to kernel...");
    let mut mmap_buf = vec![
        0;
        system_table.boot_services().memory_map_size()
            + 5 * core::mem::size_of::<uefi::table::boot::MemoryDescriptor>()
    ];
    let mut memory_map_entries = Vec::with_capacity(
        mmap_buf.capacity() / core::mem::size_of::<uefi::table::boot::MemoryDescriptor>() - 2,
    );

    for desc in system_table
        .exit_boot_services(image, &mut mmap_buf)
        .expect_success("Failed to exit boot services.")
        .1
    {
        if let Some(ent) = helpers::kaboom::mem_type_from_desc(desc) {
            memory_map_entries.push(ent)
        }
    }

    tags.push(kaboom::tags::TagType::MemoryMap(memory_map_entries.leak()));
    explosion.tags = tags.leak();

    unsafe {
        asm!(
            "cli",
            // "mov rsp, {}",
            "call {}",
            // in(reg) stack,
            in(reg) kernel_main,
            in("rdi") Box::leak(explosion),
            options(noreturn)
        )
    }
}
