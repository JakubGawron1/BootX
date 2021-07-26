const uefi = @import("std").os.uefi;
const w = @import("std").unicode.utf8ToUtf16LeStringLiteral;
const assert = @import("helpers").assert;
const findCfgTable = @import("helpers").findCfgTable;

pub const panic = @import("helpers").panic;

pub fn main() void {
    const system_table = uefi.system_table;
    const boot_services = uefi.system_table.boot_services.?;

    _ = system_table.con_out.?.reset(false);
    _ = system_table.con_out.?.clearScreen();
    _ = system_table.con_out.?.outputString(w("Welcome!\n\r"));

    // Open ESP
    var image: ?*uefi.protocols.LoadedImageProtocol = undefined;
    var fs: ?*uefi.protocols.SimpleFileSystemProtocol = undefined;
    var esp: ?*uefi.protocols.FileProtocol = undefined;
    assert(boot_services.handleProtocol(uefi.handle, &uefi.protocols.LoadedImageProtocol.guid, @ptrCast(*?*c_void, &image)), "Unable to open image info.", @src());
    assert(boot_services.handleProtocol(image.?.device_handle.?, &uefi.protocols.SimpleFileSystemProtocol.guid, @ptrCast(*?*c_void, &fs)), "Unable to open filesystem", @src());
    assert(fs.?.openVolume(&esp.?), "Unable to open volume.", @src());

    // Find ACPI pointer
    if (findCfgTable(uefi.tables.ConfigurationTable.acpi_20_table_guid)) |_| {
        _ = system_table.con_out.?.outputString(w("Found ACPI v2+.\n\r"));
    } else |_| if (findCfgTable(uefi.tables.ConfigurationTable.acpi_10_table_guid)) |_| {
        _ = system_table.con_out.?.outputString(w("Found ACPI v1.\n\r"));
    } else |_| @panic("No ACPI found.");

    // Get memory map and exit boot services
    _ = system_table.con_out.?.outputString(w("Exiting boot services..."));
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