use core::arch::asm;

use log::info;
use uefi::ResultExt;

pub fn init_output() {
    unsafe {
        uefi_services::system_table()
            .as_mut()
            .stdout()
            .reset(false)
            .expect_success("Failed to reset stdout");
        let desired_mode = uefi_services::system_table()
            .as_mut()
            .stdout()
            .modes()
            .last()
            .unwrap()
            .unwrap();
        uefi_services::system_table()
            .as_mut()
            .stdout()
            .set_mode(desired_mode)
            .expect_success("Failed to set mode");
        uefi_services::system_table()
            .as_mut()
            .stdout()
            .set_color(
                uefi::proto::console::text::Color::White,
                uefi::proto::console::text::Color::Black,
            )
            .expect_success("Failed to set color");
    }
}

pub fn setup_paging() {
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
        let pml4 = super::Pml4::get();
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
            .expect_success("Failed to get GOP protocol")
            .get()
            .as_mut()
            .unwrap()
    }
}

pub fn get_rsdp() -> &'static acpi::tables::Rsdp {
    unsafe {
        (uefi_services::system_table()
            .as_mut()
            .config_table()
            .iter()
            .find(|ent| ent.guid == uefi::table::cfg::ACPI2_GUID)
            .unwrap_or_else(|| {
                uefi_services::system_table()
                    .as_mut()
                    .config_table()
                    .iter()
                    .find(|ent| ent.guid == uefi::table::cfg::ACPI_GUID)
                    .expect("No ACPI found on the system!")
            })
            .address as *const acpi::tables::Rsdp)
            .as_ref()
            .unwrap()
    }
}
