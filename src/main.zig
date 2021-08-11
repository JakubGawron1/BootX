// Copyright (c) 2021 VisualDevelopment. All rights reserved.

const std = @import("std");
const uefi = std.os.uefi;
const w = std.unicode.utf8ToUtf16LeStringLiteral;
const helpers = @import("helpers");
const assert = helpers.assert;

pub const panic = helpers.panic;

pub fn main() noreturn {
    const system_table = uefi.system_table;
    const boot_services = uefi.system_table.boot_services.?;

    _ = system_table.con_out.?.clearScreen();
    helpers.con_out_writer.writeAll("Welcome!\n\r") catch unreachable;

    helpers.openESP();

    // Find ACPI pointer
    if (helpers.findCfgTable(uefi.tables.ConfigurationTable.acpi_20_table_guid)) |_| {
        helpers.con_out_writer.writeAll("Found ACPI v2+.\n\r") catch unreachable;
    } else |_| if (helpers.findCfgTable(uefi.tables.ConfigurationTable.acpi_10_table_guid)) |_| {
        helpers.con_out_writer.writeAll("Found ACPI v1.\n\r") catch unreachable;
    } else |_| assert(false, "No ACPI found.", @src());

    if (helpers.loadFile(w("\\Fuse.exec"), uefi.protocols.FileProtocol.efi_file_mode_read, 0)) |buffer| {
        helpers.con_out_writer.writeAll("Parsing 'Fuse.exec':\n\r") catch unreachable;
        if (std.elf.Header.parse(buffer[0..64])) |header| {
            assert(header.endian == .Little, "Only little-endian is supported.", @src());
            assert(header.is_64 == true and header.machine == ._X86_64, "Only AMD64 is supported.", @src());

            var phdr_iter = header.program_header_iterator(std.io.fixedBufferStream(buffer));
            while (if (phdr_iter.next()) |phdr| phdr else |_| @panic("Unable to parse program header entry.")) |phdr_entry| {
                if (phdr_entry.p_type != std.elf.PT_LOAD) continue;
                helpers.con_out_writer.print("    Loading data from 0x{X} to 0x{X}\n\r", .{ @ptrToInt(buffer.ptr + phdr_entry.p_offset), phdr_entry.p_vaddr }) catch unreachable;
            }
        } else |_| assert(false, "Failed to parse elf header.", @src());
    } else |_| assert(false, "Failed to open file.", @src());

    // Get memory map and exit boot services
    helpers.con_out_writer.writeAll("Exiting boot services...") catch unreachable;
    var mem_map_size: usize = 0;
    var mem_map: [*]uefi.tables.MemoryDescriptor = undefined;
    var map_key: usize = 0;
    var desc_size: usize = 0;
    var desc_ver: u32 = 0;
    _ = boot_services.getMemoryMap(&mem_map_size, mem_map, &map_key, &desc_size, &desc_ver);
    mem_map_size += desc_size;
    assert(boot_services.allocatePool(uefi.tables.MemoryType.BootServicesData, mem_map_size, @ptrCast(*[*]align(8) u8, &mem_map)), "Unable to allocate memory map.", @src());
    assert(boot_services.getMemoryMap(&mem_map_size, mem_map, &map_key, &desc_size, &desc_ver), "Unable to get memory map.", @src());

    assert(boot_services.exitBootServices(uefi.handle, map_key), "Unable to exit boot services.", @src());

    // Jump to kernel code
    // TODO: Jump to kernel code

    while (true) {}
}
