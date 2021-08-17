#![no_std]
#![no_main]
#![feature(asm)]
#![feature(abi_efiapi)]
#![feature(allocator_api)]
#![warn(unused_extern_crates)]

mod helpers;

extern crate alloc;

use alloc::*;
use elf_rs::*;
use log::*;
use uefi::prelude::*;
use uefi::proto::media::file::{File, FileAttribute, FileInfo, FileMode, FileType};
use uefi::table::boot::{AllocateType, MemoryType};
use uefi::ResultExt;

#[entry]
fn efi_main(image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).expect_success("Failed to initialize utilities");
    system_table.stdout().reset(false).expect_success("Failed to reset stdout");
    info!("Welcome...");
    let boot_services = system_table.boot_services();
    let fs = unsafe { boot_services.get_image_file_system(image).unwrap().unwrap().get().as_mut().unwrap() };
    let mut esp = fs.open_volume().unwrap().unwrap();
    let mut file = match esp
        .open("\\fuse.exec", FileMode::Read, FileAttribute::empty())
        .expect_success("Your volume is corrupted, no fuse.exec was found")
        .into_type()
        .unwrap()
        .unwrap()
    {
        FileType::Regular(f) => f,
        _ => panic!("How do you expect me to load a folder?"),
    };

    let mut buffer = vec![0; file.get_boxed_info::<FileInfo>().expect_success("Failed to get file info").file_size().try_into().unwrap()];

    file.read(&mut buffer).expect_success("Failed to read fuse.exec.");

    if let Elf::Elf64(elf) = Elf::from_bytes(&buffer).unwrap() {
        assert_eq!(elf.header().machine(), ElfMachine::x86_64);
        assert!(elf.header().entry_point() >= helpers::paging::KERNEL_VIRT_OFFSET, "Only higher-half kernels.");

        info!("Parsing program headers: ");
        for phdr in elf.program_header_iter() {
            if let ProgramType::LOAD = phdr.ph.ph_type() {
                assert!(phdr.ph.vaddr() >= helpers::paging::KERNEL_VIRT_OFFSET, "Only higher-half kernels.");

                let src = &buffer[phdr.ph.offset() as usize..phdr.ph.offset() as usize + phdr.ph.filesz() as usize];
                let dest = unsafe { core::slice::from_raw_parts_mut((phdr.ph.vaddr() - helpers::paging::KERNEL_VIRT_OFFSET) as *mut u8, phdr.ph.memsz() as usize) };
                let npages = (((phdr.ph.memsz() + 0xFFF) / 0x1000) as usize).try_into().unwrap();
                info!("vaddr: {:#X}, paddr: {:#X}, npages: {:#X}", phdr.ph.vaddr(), phdr.ph.vaddr() - helpers::paging::KERNEL_VIRT_OFFSET, npages);
                boot_services
                    .allocate_pages(AllocateType::Address((phdr.ph.vaddr() - helpers::paging::KERNEL_VIRT_OFFSET) as usize), MemoryType::custom(0x80000000), npages)
                    .expect_success("Failed to allocate section");
                dest.copy_from_slice(src);
            }
        }

        info!("Setting up higher half paging mappings:");
        info!("    1. Turning off write protection...");
        unsafe {
            asm!("mov rax, cr0",
            "and rax, {wp_bit}",
            "mov cr0, rax",
                wp_bit = const !(1u64 << 16)
            );
        }
        info!("    2.  Modifying paging mappings to map higher-half...");
        unsafe {
            let mut pml4: *mut helpers::paging::PageTable;
            asm!("mov {}, cr3", out(reg) pml4);
            helpers::paging::map_higher_half(pml4);
        }

        info!("Exiting boot services and jumping to kernel...");
        let mut mmap_buf = vec![0; boot_services.memory_map_size()];
        mmap_buf.resize(boot_services.memory_map_size(), 0);
        system_table.exit_boot_services(image, &mut mmap_buf).expect_success("Failed to exit boot services.");

        unsafe {
            core::mem::transmute::<*const (), fn() -> !>(elf.header().entry_point() as *const ())();
        };
    } else {
        panic!("Only AMD64, I'm not saying please.");
    }
}
