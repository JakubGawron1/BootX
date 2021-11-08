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

extern crate alloc;

use alloc::{boxed::Box, vec, vec::Vec};
use core::{cell::UnsafeCell, mem::size_of};

use amd64::paging::PML4;
use log::*;
use uefi::{
    prelude::{entry, Boot, Handle, ResultExt, Status, SystemTable},
    proto::media::file::{FileAttribute, FileMode},
    table::boot::{AllocateType, MemoryType},
};

mod helpers;

#[entry]
fn efi_main(image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).expect_success("Failed to initialize utilities");
    system_table
        .stdout()
        .reset(false)
        .expect_success("Failed to reset stdout");
    let desired_mode = system_table.stdout().modes().last().unwrap().unwrap();
    system_table
        .stdout()
        .set_mode(desired_mode)
        .expect_success("Failed to set mode");
    system_table
        .stdout()
        .set_color(
            uefi::proto::console::text::Color::White,
            uefi::proto::console::text::Color::Black,
        )
        .expect_success("Failed to set color");

    info!("Welcome...");

    let esp = helpers::open_esp(image);

    let buffer = helpers::load_file(esp, "\\System\\fuse.exec", FileMode::Read, FileAttribute::empty());

    let elf = goblin::elf::Elf::parse(&buffer).expect("Failed to parse kernel elf");

    info!("{:X?}", elf.header);
    assert!(elf.is_64, "Only ELF64");
    assert_eq!(elf.header.e_machine, goblin::elf::header::EM_X86_64);
    assert!(elf.little_endian, "Only little-endian ELFs");
    assert!(
        elf.entry >= amd64::paging::KERNEL_VIRT_OFFSET,
        "Only higher-half kernels"
    );

    info!("Parsing program headers: ");
    for phdr in elf
        .program_headers
        .iter()
        .filter(|phdr| phdr.p_type == goblin::elf::program_header::PT_LOAD)
    {
        assert!(
            phdr.p_vaddr >= amd64::paging::KERNEL_VIRT_OFFSET,
            "Only higher-half kernels."
        );

        let offset: usize = phdr.p_offset.try_into().unwrap();
        let memsz = phdr.p_memsz.try_into().unwrap();
        let file_size: usize = phdr.p_filesz.try_into().unwrap();
        let src = &buffer[offset..(offset + file_size)];
        let dest = unsafe {
            core::slice::from_raw_parts_mut(
                (phdr.p_vaddr - amd64::paging::KERNEL_VIRT_OFFSET) as *mut u8,
                memsz,
            )
        };
        let npages = (memsz + 0xFFF) as usize / 0x1000;
        info!(
            "vaddr: {:#X}, paddr: {:#X}, npages: {:#X}",
            phdr.p_vaddr,
            phdr.p_vaddr - amd64::paging::KERNEL_VIRT_OFFSET,
            npages
        );
        assert_eq!(
            system_table
                .boot_services()
                .allocate_pages(
                    AllocateType::Address(
                        (phdr.p_vaddr - amd64::paging::KERNEL_VIRT_OFFSET)
                            .try_into()
                            .unwrap(),
                    ),
                    MemoryType::LOADER_DATA,
                    npages,
                )
                .expect_success("Failed to load section above. Sections might be misaligned."),
            phdr.p_vaddr - amd64::paging::KERNEL_VIRT_OFFSET
        );

        for (a, b) in dest
            .iter_mut()
            .zip(src.iter().chain(core::iter::repeat(&0)))
        {
            *a = *b
        }
    }

    info!("Setting up higher-half paging mappings:");
    info!("    1. Turning off write protection...");

    unsafe {
        asm!("mov rax, cr0",
        "and rax, {wp_bit}",
        "mov cr0, rax",
        wp_bit = const !(1u64 << 16)
        );
    }

    info!("    2. Modifying paging mappings to map higher-half...");

    unsafe {
        let pml4 = <amd64::paging::PageTable as PML4>::get();
        pml4.map_higher_half();
        pml4.set();
    }

    let bss = elf
        .section_headers
        .iter()
        .find(|shdr| elf.shdr_strtab.get_at(shdr.sh_name).unwrap_or_default() == ".bss")
        .expect("No .bss section found");
    unsafe {
        (bss.sh_addr as *mut u8).write_bytes(0, bss.sh_size.try_into().unwrap());
    }

    let mut explosion = Box::new(kaboom::ExplosionResult::new(Default::default()));
    let mut tags = Vec::with_capacity(3);

    let gop = unsafe {
        system_table
            .boot_services()
            .locate_protocol::<uefi::proto::console::gop::GraphicsOutput>()
            .expect_success("Failed to get GOP protocol")
            .get()
            .as_mut()
            .unwrap()
    };

    let frame_buffer = Box::new(kaboom::tags::FrameBufferInfo {
        resolution: kaboom::tags::ScreenRes::new(gop.current_mode_info().resolution()),
        pixel_format: match gop.current_mode_info().pixel_format() {
            uefi::proto::console::gop::PixelFormat::Rgb => kaboom::tags::PixelFormat::Rgb,
            uefi::proto::console::gop::PixelFormat::Bgr => kaboom::tags::PixelFormat::Bgr,
            uefi::proto::console::gop::PixelFormat::Bitmask => kaboom::tags::PixelFormat::Bitmask,
            _ => panic!("Blt-only mode not supported."),
        },
        pixel_bitmask: gop.current_mode_info().pixel_bitmask().map(|v| {
            kaboom::tags::PixelBitmask {
                red: v.red,
                green: v.green,
                blue: v.blue,
            }
        }),
        pixels_per_scanline: gop.current_mode_info().stride(),
        base: gop.frame_buffer().as_mut_ptr() as *mut u32,
    });
    tags.push(kaboom::tags::TagType::FrameBuffer(Box::leak(frame_buffer)));

    let rsdp = unsafe {
        (system_table
            .config_table()
            .iter()
            .find(|ent| ent.guid == uefi::table::cfg::ACPI2_GUID)
            .unwrap_or_else(|| {
                system_table
                    .config_table()
                    .iter()
                    .find(|ent| ent.guid == uefi::table::cfg::ACPI_GUID)
                    .expect("No ACPI found on the system!")
            })
            .address as *const acpi::tables::Rsdp)
            .as_ref()
            .unwrap()
    };

    tags.push(kaboom::tags::TagType::ACPI(rsdp));

    info!("{:#X?}", rsdp);
    info!("{:#X?}", explosion);

    info!("Exiting boot services and jumping to kernel...");
    let mut mmap_buf = vec![0; system_table.boot_services().memory_map_size()];
    let mut memory_map_entries = Vec::with_capacity(
        mmap_buf.capacity() / size_of::<uefi::table::boot::MemoryDescriptor>() + 1,
    );
    mmap_buf.resize(system_table.boot_services().memory_map_size(), 0);

    for desc in system_table
        .exit_boot_services(image, &mut mmap_buf)
        .expect_success("Failed to exit boot services.")
        .1
    {
        match desc.ty {
            MemoryType::CONVENTIONAL => {
                memory_map_entries.push(UnsafeCell::new(kaboom::tags::MemoryEntry::Usable(
                    kaboom::tags::MemoryData {
                        base: desc.phys_start,
                        pages: desc.page_count,
                    },
                )))
            }
            MemoryType::LOADER_CODE | MemoryType::LOADER_DATA => {
                memory_map_entries.push(UnsafeCell::new(
                    kaboom::tags::MemoryEntry::BootLoaderReclaimable(kaboom::tags::MemoryData {
                        base: desc.phys_start,
                        pages: desc.page_count,
                    }),
                ))
            }
            MemoryType::ACPI_RECLAIM => {
                memory_map_entries.push(UnsafeCell::new(
                    kaboom::tags::MemoryEntry::ACPIReclaimable(kaboom::tags::MemoryData {
                        base: desc.phys_start,
                        pages: desc.page_count,
                    }),
                ))
            }
            _ => {}
        }
    }

    tags.push(kaboom::tags::TagType::MemoryMap(memory_map_entries.leak()));
    explosion.tags = tags.leak();

    unsafe { asm!("cli") }

    let kernel_main: kaboom::EntryPoint = unsafe { core::mem::transmute(elf.entry as *const ()) };

    kernel_main(Box::leak(explosion));
}
