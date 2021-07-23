const uefi = @import("std").os.uefi;
const w = @import("std").unicode.utf8ToUtf16LeStringLiteral;
const assert = @import("helpers").assert;
const builtin = @import("std").builtin;
const fmt = @import("std").fmt;
const utf8ToUtf16Le = @import("std").unicode.utf8ToUtf16Le;
const debug = @import("std").debug;

pub fn panic(msg: []const u8, stack_trace: ?*builtin.StackTrace) noreturn {
    _ = stack_trace;
    var buf = [_:0]u8{0} ** 512;
    var buf16 = [_:0]u16{0} ** 512;
    _ = fmt.bufPrintZ(buf[0..], "Panic: {s}\n\rStack trace:\n\r\t0x{X}\n\r", .{ msg, @returnAddress() }) catch unreachable;
    _ = utf8ToUtf16Le(buf16[0..], buf[0..]) catch unreachable;
    _ = uefi.system_table.con_out.?.outputString(buf16[0..]);
    var iter = debug.StackIterator.init(@returnAddress(), null);
    while (iter.next()) |addr| {
        _ = fmt.bufPrintZ(buf[0..], "\t0x{X}\n\r", .{addr}) catch unreachable;
        _ = utf8ToUtf16Le(buf16[0..], buf[0..]) catch unreachable;
        _ = uefi.system_table.con_out.?.outputString(buf16[0..]);
    }
    while (true) {}
}

pub fn findCfgTable(guid: uefi.Guid) !uefi.tables.ConfigurationTable {
    var i: usize = 0;
    while (i < uefi.system_table.number_of_table_entries) : (i += 1) {
        const table = uefi.system_table.configuration_table[i];

        if (guid.eql(table.vendor_guid)) return table;
    }

    return error.CfgTableNotFound;
}

pub fn main() void {
    const system_table = uefi.system_table;
    const boot_services = uefi.system_table.boot_services.?;

    _ = system_table.con_out.?.reset(false);
    _ = system_table.con_out.?.clearScreen();
    _ = system_table.con_out.?.outputString(w("Welcome!\n\r"));

    // Open ESP
    var image: ?*uefi.protocols.LoadedImageProtocol = undefined;
    assert(boot_services.handleProtocol(uefi.handle, &uefi.protocols.LoadedImageProtocol.guid, @ptrCast(*?*c_void, &image)), "Unable to open image info.", @src());
    var fs: ?*uefi.protocols.SimpleFileSystemProtocol = undefined;
    assert(boot_services.handleProtocol(image.?.device_handle.?, &uefi.protocols.SimpleFileSystemProtocol.guid, @ptrCast(*?*c_void, &fs)), "Unable to open filesystem", @src());
    var esp: ?*uefi.protocols.FileProtocol = undefined;
    assert(fs.?.openVolume(&esp.?), "Unable to open volume.", @src());

    // Find ACPI pointer
    if (findCfgTable(uefi.tables.ConfigurationTable.acpi_20_table_guid)) |_| {
        _ = system_table.con_out.?.outputString(w("Found ACPI v2+.\n\r"));
    } else |_| if (findCfgTable(uefi.tables.ConfigurationTable.acpi_10_table_guid)) |_| {
        _ = system_table.con_out.?.outputString(w("Found ACPI v1.\n\r"));
    } else |_| @panic("No ACPI found.");

    _ = system_table.con_out.?.outputString(w("Exiting boot services..."));
    var mem_map_size: usize = undefined;
    var mem_map: [*]uefi.tables.MemoryDescriptor = undefined;
    var map_key: usize = undefined;
    var desc_size: usize = undefined;
    var desc_ver: u32 = undefined;
    // Get memory map size
    _ = boot_services.getMemoryMap(&mem_map_size, mem_map, &map_key, &desc_size, &desc_ver);
    // Add new descriptor
    mem_map_size += desc_size;
    assert(boot_services.allocatePool(uefi.tables.MemoryType.BootServicesData, mem_map_size, @ptrCast(*[*]align(8) u8, &mem_map)), "Unable to allocate memory map.", @src());
    assert(boot_services.getMemoryMap(&mem_map_size, mem_map, &map_key, &desc_size, &desc_ver), "Unable to get memory map.", @src());
    // Exit boot services
    assert(boot_services.exitBootServices(uefi.handle, map_key), "Unable to exit boot services.", @src());

    // Jump to kernel code
    // TODO: Jump to kernel code

    while (true) {}
}