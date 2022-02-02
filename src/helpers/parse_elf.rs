/*
 * Copyright (c) VisualDevelopment 2021-2022.
 * This project is licensed by the Creative Commons Attribution-NoCommercial-NoDerivatives licence.
 */

use log::info;
use uefi::ResultExt;

pub fn parse_elf(
    mem_mgr: &mut super::mem::MemoryManager,
    buffer: &[u8],
) -> (kaboom::EntryPoint, *const u8) {
    let elf = goblin::elf::Elf::parse(buffer).expect("Failed to parse kernel elf");

    info!("{:X?}", elf.header);
    assert!(elf.is_64, "Only ELF64");
    assert_eq!(elf.header.e_machine, goblin::elf::header::EM_X86_64);
    assert!(elf.little_endian, "Only little-endian ELFs");
    assert!(
        elf.entry as usize >= amd64::paging::KERNEL_VIRT_OFFSET,
        "Only higher-half kernels"
    );

    info!("Parsing program headers: ");
    for phdr in elf
        .program_headers
        .iter()
        .filter(|phdr| phdr.p_type == goblin::elf::program_header::PT_LOAD)
    {
        assert!(
            phdr.p_vaddr as usize >= amd64::paging::KERNEL_VIRT_OFFSET,
            "Only higher-half kernels."
        );

        let offset: usize = phdr.p_offset.try_into().unwrap();
        let memsz = phdr.p_memsz.try_into().unwrap();
        let file_size: usize = phdr.p_filesz.try_into().unwrap();
        let src = &buffer[offset..(offset + file_size)];
        let dest = unsafe {
            core::slice::from_raw_parts_mut(
                (phdr.p_vaddr as usize - amd64::paging::KERNEL_VIRT_OFFSET) as *mut u8,
                memsz,
            )
        };
        let npages = (memsz + 0xFFF) as usize / 0x1000;
        info!(
            "vaddr: {:#X}, paddr: {:#X}, npages: {:#X}",
            phdr.p_vaddr,
            phdr.p_vaddr as usize - amd64::paging::KERNEL_VIRT_OFFSET,
            npages
        );
        assert_eq!(
            unsafe { uefi_services::system_table().as_mut() }
                .boot_services()
                .allocate_pages(
                    uefi::table::boot::AllocateType::Address(
                        phdr.p_vaddr as usize - amd64::paging::KERNEL_VIRT_OFFSET,
                    ),
                    uefi::table::boot::MemoryType::LOADER_DATA,
                    npages,
                )
                .expect_success("Failed to load section above. Sections might be misaligned.")
                as usize,
            phdr.p_vaddr as usize - amd64::paging::KERNEL_VIRT_OFFSET
        );

        mem_mgr.allocate((
            phdr.p_vaddr as usize - amd64::paging::KERNEL_VIRT_OFFSET,
            npages,
        ));

        for (a, b) in dest
            .iter_mut()
            .zip(src.iter().chain(core::iter::repeat(&0)))
        {
            *a = *b
        }
    }

    let bss = elf
        .section_headers
        .iter()
        .find(|shdr| elf.shdr_strtab.get_at(shdr.sh_name).unwrap_or_default() == ".bss")
        .expect("No .bss section found");
    unsafe {
        (bss.sh_addr as *mut u8).write_bytes(0, bss.sh_size.try_into().unwrap());
    }

    let explosion_fuel = unsafe {
        (elf.section_headers
            .iter()
            .find(|shdr| elf.shdr_strtab.get_at(shdr.sh_name).unwrap_or_default() == ".kaboom")
            .expect("No .kaboom section found")
            .sh_addr as *const kaboom::ExplosionFuel)
            .as_ref()
            .unwrap()
    };

    info!("{:#X?}", explosion_fuel);

    (
        unsafe { core::mem::transmute(elf.entry as *const ()) },
        explosion_fuel.stack,
    )
}
