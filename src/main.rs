//! Copyright (c) ChefKiss Inc 2021-2022.
//! This project is licensed by the Creative Commons Attribution-NoCommercial-NoDerivatives license.

#![no_std]
#![no_main]
#![deny(warnings, unused_extern_crates, clippy::cargo, rust_2021_compatibility)]
#![feature(abi_efiapi, allocator_api, asm_const, used_with_arg)]

extern crate alloc;

mod helpers;

use alloc::{boxed::Box, vec, vec::Vec};
use core::arch::asm;

use log::{info, trace};
use uefi::{
    prelude::*,
    proto::media::file::{FileAttribute, FileMode},
};

#[no_mangle]
pub extern "efiapi" fn efi_main(image: Handle, mut system_table: SystemTable<Boot>) {
    uefi_services::init(&mut system_table).expect("Failed to initialize utilities");
    helpers::setup::init_output();
    info!("Welcome...");
    helpers::setup::setup_paging();

    let mut esp = helpers::file::open_esp(image);

    let buffer = helpers::file::load(
        &mut esp,
        cstr16!("\\System\\cardboard.exec"),
        FileMode::Read,
        FileAttribute::empty(),
    )
    .leak();

    let mod_buffer = helpers::file::load(
        &mut esp,
        cstr16!("\\System\\test.raw"),
        FileMode::Read,
        FileAttribute::empty(),
    )
    .leak();
    trace!("{:#X?}", mod_buffer.as_ptr());

    let mut mem_mgr = helpers::mem::MemoryManager::new();
    mem_mgr.allocate((mod_buffer.as_ptr() as usize, mod_buffer.len()));

    let kernel_main = helpers::parse_elf::parse_elf(&mut mem_mgr, buffer);

    let mut stack = Vec::new();
    stack.resize(0x14000, 0u8);
    let stack = (stack.leak().as_ptr() as usize + amd64::paging::KERNEL_VIRT_OFFSET) as *const u8;
    mem_mgr.allocate((stack as usize - amd64::paging::KERNEL_VIRT_OFFSET, 0x2000));

    let fbinfo = helpers::phys_to_kern_ref(Box::leak(helpers::fb::fbinfo_from_gop(
        helpers::setup::get_gop(),
    )));
    let rsdp = helpers::setup::get_rsdp();

    let mut explosion = Box::new(sulfur_dioxide::BootInfo::new(
        buffer,
        sulfur_dioxide::SpecialisedSettings {
            verbose: cfg!(debug_assertions),
        },
        Some(fbinfo),
        rsdp,
    ));

    let modules = vec![sulfur_dioxide::module::Module::Audio(
        sulfur_dioxide::module::ModuleInner {
            name: core::str::from_utf8(helpers::phys_to_kern_slice_ref("testaudio".as_bytes()))
                .unwrap(),
            data: helpers::phys_to_kern_slice_ref(mod_buffer),
        },
    )];

    trace!("{:#X?}", explosion.as_ref() as *const _);

    info!("Exiting boot services and jumping to kernel...");
    let sizes = system_table.boot_services().memory_map_size();
    let mut mmap_buf = vec![0; sizes.map_size + 2 * sizes.entry_size];
    let mut memory_map_entries = Vec::with_capacity(
        mmap_buf.capacity() / core::mem::size_of::<uefi::table::boot::MemoryDescriptor>() - 2,
    );

    system_table
        .exit_boot_services(image, &mut mmap_buf)
        .expect("Failed to exit boot services.")
        .1
        .for_each(|v| {
            if let Some(v) = mem_mgr.mem_type_from_desc(v) {
                memory_map_entries.push(v);
            }
        });

    explosion.memory_map = helpers::phys_to_kern_slice_ref(memory_map_entries.leak());
    explosion.modules = modules.leak();

    unsafe {
        asm!(
            "cli",
            "mov rsp, {}",
            "call {}",
            in(reg) stack,
            in(reg) kernel_main,
            in("rdi") helpers::phys_to_kern_ref(Box::leak(explosion)),
            options(noreturn)
        )
    }
}
