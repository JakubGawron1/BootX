//! Copyright (c) VisualDevelopment 2021-2022.
//! This project is licensed by the Creative Commons Attribution-NoCommercial-NoDerivatives licence.

use core::arch::asm;

use acpi::tables::rsdp::RSDP;
use amd64::paging::pml4::PML4;
use log::debug;
use uefi::proto::console::text::Color;

pub fn init_output() {
    unsafe {
        let stdout = uefi_services::system_table().as_mut().stdout();
        stdout.reset(false).expect("Failed to reset stdout");
        let desired_mode = stdout.modes().last().unwrap();
        stdout.set_mode(desired_mode).expect("Failed to set mode");
        stdout
            .set_color(Color::White, Color::Black)
            .expect("Failed to set color");
        stdout.clear().expect("Failed to clear console");
    }
}

pub fn setup_paging() {
    debug!("Setting up higher-half paging mappings:");
    debug!("    1. Turning off write protection...");

    unsafe {
        asm!("mov rax, cr0",
        "and rax, {wp_bit}",
        "mov cr0, rax",
        wp_bit = const !(1u64 << 16)
        );
    }

    debug!("    2. Modifying paging mappings to map higher-half...");

    unsafe {
        let pml4 = super::PML4::get();
        pml4.map_higher_half();
        pml4.set();
    }
}

pub fn get_gop() -> &'static mut uefi::proto::console::gop::GraphicsOutput<'static> {
    unsafe {
        uefi_services::system_table()
            .as_mut()
            .boot_services()
            .locate_protocol::<uefi::proto::console::gop::GraphicsOutput>()
            .expect("Failed to get GOP protocol")
            .get()
            .as_mut()
            .unwrap()
    }
}

pub fn get_rsdp() -> &'static RSDP {
    let mut iter = unsafe { uefi_services::system_table().as_mut().config_table().iter() };
    let rsdp: *const RSDP = iter
        .find(|ent| ent.guid == uefi::table::cfg::ACPI2_GUID)
        .unwrap_or_else(|| {
            iter.find(|ent| ent.guid == uefi::table::cfg::ACPI_GUID)
                .expect("No ACPI found on the system!")
        })
        .address as _;
    super::phys_to_kern_ref(unsafe { &*rsdp })
}
