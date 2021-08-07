// Copyright (c) 2021 VisualDevelopment. All rights reserved.
const uefi = @import("std").os.uefi;
const w = @import("std").unicode.utf8ToUtf16LeStringLiteral;
const assert = @import("helpers").assert;
const findCfgTable = @import("helpers").findCfgTable;
const openESP = @import("helpers").openESP;
const loadFile = @import("helpers").loadFile;
const con_out_writer = @import("helpers").con_out_writer;

pub const panic = @import("helpers").panic;

pub fn main() void {
    const system_table = uefi.system_table;
    const boot_services = uefi.system_table.boot_services.?;

    _ = system_table.con_out.?.clearScreen();
    con_out_writer.writeAll("Welcome!\n\r") catch unreachable;

    openESP();
    const read_mode = uefi.protocols.FileProtocol.efi_file_mode_read;
    if (loadFile(w("\\text.txt"), read_mode, 0)) |ret| {
        con_out_writer.print("Reading text.txt: {s}\r", .{ret.buffer[0..ret.buffer_size]}) catch unreachable;
    } else |_| assert(false, "Failed to open file.", @src());

    // Find ACPI pointer
    if (findCfgTable(uefi.tables.ConfigurationTable.acpi_20_table_guid)) |_| {
        con_out_writer.writeAll("Found ACPI v2+.\n\r") catch unreachable;
    } else |_| if (findCfgTable(uefi.tables.ConfigurationTable.acpi_10_table_guid)) |_| {
        con_out_writer.writeAll("Found ACPI v1.\n\r") catch unreachable;
    } else |_| assert(false, "No ACPI found.", @src());

    // Get memory map and exit boot services
    con_out_writer.writeAll("Exiting boot services...") catch unreachable;
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