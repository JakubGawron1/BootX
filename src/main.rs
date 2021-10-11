#![no_std]
#![no_main]
#![feature(asm)]
#![feature(abi_efiapi)]
#![feature(allocator_api)]
#![warn(unused_extern_crates)]

mod helpers;

extern crate alloc;

use core::{mem::size_of, ptr::null};

use alloc::{boxed::Box, vec, vec::Vec};
use elf_rs::*;
use log::*;
use uefi::{
    prelude::*,
    table::boot::{AllocateType, MemoryType},
    ResultExt,
};

#[entry]
fn efi_main(image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).expect_success("Failed to initialize utilities");
    system_table.stdout().reset(false).expect_success("Failed to reset stdout");
    system_table.stdout().set_mode(system_table.stdout().modes().last().unwrap().unwrap()).expect_success("Failed to set mode");
    system_table
        .stdout()
        .set_color(uefi::proto::console::text::Color::White, uefi::proto::console::text::Color::Black)
        .expect_success("Failed to set color");

    info!("Welcome...");

    helpers::open_esp(image);

    let buffer = helpers::load_file("\\Fuse.exec");

    if let Elf::Elf64(elf) = Elf::from_bytes(&buffer).unwrap() {
        debug!("{:X?}", elf.header());
        assert_eq!(elf.header().machine(), ElfMachine::x86_64);
        assert!(elf.header().entry_point() >= helpers::paging::KERNEL_VIRT_OFFSET, "Only higher-half kernels.");

        let explosion_info_section = elf.lookup_section(".kaboom").expect("No .kaboom section found");
        debug!("{:X?}", explosion_info_section);

        info!("Parsing program headers: ");
        for phdr in elf.program_header_iter() {
            if let ProgramType::LOAD = phdr.ph.ph_type() {
                assert!(phdr.ph.vaddr() >= helpers::paging::KERNEL_VIRT_OFFSET, "Only higher-half kernels.");

                let offset: usize = phdr.ph.offset().try_into().unwrap();
                let memsz = phdr.ph.memsz().try_into().unwrap();
                let src = &buffer[offset..(offset + phdr.ph.filesz() as usize)];
                let dest = unsafe { core::slice::from_raw_parts_mut((phdr.ph.vaddr() - helpers::paging::KERNEL_VIRT_OFFSET) as *mut u8, memsz) };
                let npages = (phdr.ph.memsz() + 0xFFF) as usize / 0x1000;
                info!("vaddr: {:#X}, paddr: {:#X}, npages: {:#X}", phdr.ph.vaddr(), phdr.ph.vaddr() - helpers::paging::KERNEL_VIRT_OFFSET, npages);
                match system_table
                    .boot_services()
                    .allocate_pages(AllocateType::Address((phdr.ph.vaddr() - helpers::paging::KERNEL_VIRT_OFFSET).try_into().unwrap()), MemoryType::LOADER_DATA, npages)
                {
                    Ok(_) => {}
                    Err(_) => warn!("Potentially failed to load a section; binary may not run!"),
                };
                dest.fill(0);

                for (a, b) in dest.iter_mut().zip(src.iter()) {
                    *a = *b
                }
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
            let mut pml4: *mut helpers::paging::PageTable;
            asm!("mov {}, cr3", out(reg) pml4);
            helpers::paging::map_higher_half(pml4);
            asm!("mov cr3, {}", in(reg) pml4);
        }

        let explosion_info = unsafe { &*(explosion_info_section.sh.addr() as *const kaboom::ExplosionInfo) };
        debug!("{:X?}", explosion_info);
        assert!(explosion_info.stack != null(), "Stack pointer is null");
        assert!((explosion_info.stack as u64 & (1u64 << 63)) != 0, "Stack pointer is non-canonical");

        let gop = unsafe {
            system_table
                .boot_services()
                .locate_protocol::<uefi::proto::console::gop::GraphicsOutput>()
                .expect_success("Failed to get GOP protocol")
                .get()
                .as_mut()
                .unwrap()
        };

        let mut explosion = Box::new(kaboom::ExplosionResult::new(Default::default()));
        let mut tags: Vec<&dyn kaboom::tags::Tag> = Vec::with_capacity(2);
        let mut memory_map = Box::new(kaboom::tags::MemoryMap::new());
        let pixel_bitmask = gop.current_mode_info().pixel_bitmask();
        let frame_buffer = Box::new(kaboom::tags::FrameBufferInfo {
            resolution: kaboom::tags::ScreenRes::new(gop.current_mode_info().resolution()),
            pixel_format: match gop.current_mode_info().pixel_format() {
                uefi::proto::console::gop::PixelFormat::Rgb => kaboom::tags::PixelFormat::Rgb,
                uefi::proto::console::gop::PixelFormat::Bgr => kaboom::tags::PixelFormat::Bgr,
                uefi::proto::console::gop::PixelFormat::Bitmask => kaboom::tags::PixelFormat::Bitmask,
                _ => panic!("Blt-only mode not supported."),
            },
            pixel_bitmask: match pixel_bitmask {
                Some(v) => Some(kaboom::tags::PixelBitmask { red: v.red, green: v.green, blue: v.blue }),
                None => None,
            },
            pixels_per_scanline: gop.current_mode_info().stride(),
            base: gop.frame_buffer().as_mut_ptr() as *mut u32,
        });
        info!("Exiting boot services and jumping to kernel...");
        let mut mmap_buf = vec![0; system_table.boot_services().memory_map_size()];
        let mut memory_map_entries = Vec::with_capacity(mmap_buf.capacity() / size_of::<uefi::table::boot::MemoryDescriptor>() + 1);
        mmap_buf.resize(system_table.boot_services().memory_map_size(), 0);

        for desc in system_table.exit_boot_services(image, &mut mmap_buf).expect_success("Failed to exit boot services.").1 {
            match desc.ty {
                MemoryType::CONVENTIONAL => memory_map_entries.push(kaboom::tags::MemoryEntry::Usable(kaboom::tags::MemoryData {
                    base: desc.phys_start,
                    pages: desc.page_count,
                })),
                MemoryType::LOADER_CODE | MemoryType::LOADER_DATA => memory_map_entries.push(kaboom::tags::MemoryEntry::BootLoaderReclaimable(kaboom::tags::MemoryData {
                    base: desc.phys_start,
                    pages: desc.page_count,
                })),
                MemoryType::ACPI_RECLAIM => memory_map_entries.push(kaboom::tags::MemoryEntry::ACPIReclaimable(kaboom::tags::MemoryData {
                    base: desc.phys_start,
                    pages: desc.page_count,
                })),
                MemoryType::ACPI_NON_VOLATILE => memory_map_entries.push(kaboom::tags::MemoryEntry::ACPINVS(kaboom::tags::MemoryData {
                    base: desc.phys_start,
                    pages: desc.page_count,
                })),
                _ => {}
            }
        }
        memory_map.entries = &memory_map_entries;
        tags.push(&*memory_map);
        tags.push(&*frame_buffer);
        explosion.tags = &tags;

        unsafe {
            let entry_point = core::mem::transmute::<_, fn(&'static kaboom::ExplosionResult) -> !>(elf.header().entry_point());
            asm!("cli", "mov rsp, {}", "call {}", in(reg) explosion_info.stack, in(reg) entry_point, in("rdi") &*explosion);
        };

        loop {}
    }

    panic!("Only AMD64, I'm not saying please.");
}
