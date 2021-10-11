#![no_std]
#![no_main]
#![warn(unused_extern_crates)]
#![feature(asm)]
#![feature(abi_efiapi)]
#![feature(allocator_api)]
#![feature(core_intrinsics)]

mod helpers;

extern crate alloc;

use core::mem::size_of;

use alloc::{boxed::Box, vec, vec::Vec};
use amd64::paging::PML4;
use log::*;
use uefi::{
    prelude::{entry, Boot, Handle, ResultExt, Status, SystemTable},
    proto::media::file::{FileAttribute, FileMode},
    table::boot::{AllocateType, MemoryType},
};

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

    helpers::open_esp(image);

    let buffer = helpers::load_file("\\fuse.exec", FileMode::Read, FileAttribute::empty());

    let elf = xmas_elf::ElfFile::new(&buffer).expect("Failed to parse kernel ELF");
    xmas_elf::header::sanity_check(&elf).expect("ELF data failed sanity check");

    info!("{:X?}", elf.header);
    assert_eq!(elf.header.pt1.class(), xmas_elf::header::Class::SixtyFour);
    assert_eq!(elf.header.pt1.data(), xmas_elf::header::Data::LittleEndian);
    assert_eq!(
        elf.header.pt2.machine().as_machine(),
        xmas_elf::header::Machine::X86_64
    );
    assert!(
        elf.header.pt2.entry_point() >= amd64::paging::KERNEL_VIRT_OFFSET,
        "Only higher-half kernels"
    );

    info!("Parsing program headers: ");
    for phdr in elf
        .program_iter()
        .filter(|phdr| phdr.get_type().unwrap() == xmas_elf::program::Type::Load)
    {
        xmas_elf::program::sanity_check(phdr, &elf).expect("Program section failed sanity check");
        assert!(
            phdr.virtual_addr() >= amd64::paging::KERNEL_VIRT_OFFSET,
            "Only higher-half kernels."
        );

        let offset: usize = phdr.offset().try_into().unwrap();
        let memsz = phdr.mem_size().try_into().unwrap();
        let file_size: usize = phdr.file_size().try_into().unwrap();
        let src = &buffer[offset..(offset + file_size)];
        let dest = unsafe {
            core::slice::from_raw_parts_mut(
                (phdr.virtual_addr() - amd64::paging::KERNEL_VIRT_OFFSET) as *mut u8,
                memsz,
            )
        };
        let npages = (phdr.mem_size() + 0xFFF) as usize / 0x1000;
        info!(
            "vaddr: {:#X}, paddr: {:#X}, npages: {:#X}",
            phdr.virtual_addr(),
            phdr.virtual_addr() - amd64::paging::KERNEL_VIRT_OFFSET,
            npages
        );
        assert_eq!(
            system_table
                .boot_services()
                .allocate_pages(
                    AllocateType::Address(
                        (phdr.virtual_addr() - amd64::paging::KERNEL_VIRT_OFFSET)
                            .try_into()
                            .unwrap(),
                    ),
                    MemoryType::LOADER_DATA,
                    npages,
                )
                .expect_success("Failed to load section above. Sections might be misaligned."),
            phdr.virtual_addr() - amd64::paging::KERNEL_VIRT_OFFSET
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

    let pml4 = <amd64::paging::PageTable as PML4>::get();
    pml4.map_higher_half();
    pml4.set();

    // See issue https://github.com/nrc/xmas-elf/issues/75
    // let bss = elf
    //     .find_section_by_name(".bss")
    //     .expect("No .bss section found");
    // unsafe {
    //     (bss.address() as *mut u8).write_bytes(0, bss.size().try_into().unwrap());
    // }

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
        pixel_bitmask: match gop.current_mode_info().pixel_bitmask() {
            Some(v) => Some(kaboom::tags::PixelBitmask {
                red: v.red,
                green: v.green,
                blue: v.blue,
            }),
            None => None,
        },
        pixels_per_scanline: gop.current_mode_info().stride(),
        base: gop.frame_buffer().as_mut_ptr() as *mut u32,
    });
    tags.push(kaboom::tags::TagType::FrameBuffer(Box::leak(frame_buffer)));

    let rsdp = unsafe {
        (system_table
            .config_table()
            .into_iter()
            .find(|ent| ent.guid == uefi::table::cfg::ACPI2_GUID)
            .unwrap_or(
                system_table
                    .config_table()
                    .into_iter()
                    .find(|ent| ent.guid == uefi::table::cfg::ACPI_GUID)
                    .expect("No ACPI found on the system!"),
            )
            .address as *const acpi::tables::RSDP)
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
            MemoryType::CONVENTIONAL => memory_map_entries.push(Some(
                kaboom::tags::MemoryEntry::Usable(kaboom::tags::MemoryData {
                    base: desc.phys_start,
                    pages: desc.page_count,
                }),
            )),
            MemoryType::LOADER_CODE | MemoryType::LOADER_DATA => memory_map_entries.push(Some(
                kaboom::tags::MemoryEntry::BootLoaderReclaimable(kaboom::tags::MemoryData {
                    base: desc.phys_start,
                    pages: desc.page_count,
                }),
            )),
            MemoryType::ACPI_RECLAIM => memory_map_entries.push(Some(
                kaboom::tags::MemoryEntry::ACPIReclaimable(kaboom::tags::MemoryData {
                    base: desc.phys_start,
                    pages: desc.page_count,
                }),
            )),
            _ => {}
        }
    }

    for _ in 0..(memory_map_entries.capacity() - memory_map_entries.len()) {
        memory_map_entries.push(None);
    }

    tags.push(kaboom::tags::TagType::MemoryMap(memory_map_entries.leak()));
    explosion.tags = tags.leak();

    unsafe { asm!("cli") }

    let kernel_main = unsafe {
        core::mem::transmute::<_, kaboom::EntryPoint>(elf.header.pt2.entry_point() as *const ())
    };

    kernel_main(Box::leak(explosion));
}
