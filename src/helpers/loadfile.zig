// Copyright (c) 2021 VisualDevelopment. All rights reserved.
const uefi = @import("std").os.uefi;
const assert = @import("assert.zig").assert;
const con_out_writer = @import("conoutwriter.zig").con_out_writer;

pub var esp: *uefi.protocols.FileProtocol = undefined;

pub fn openESP() void {
    const boot_services = uefi.system_table.boot_services.?;
    var image: ?*uefi.protocols.LoadedImageProtocol = undefined;
    var fs: ?*uefi.protocols.SimpleFileSystemProtocol = undefined;
    assert(boot_services.handleProtocol(uefi.handle, &uefi.protocols.LoadedImageProtocol.guid, @ptrCast(*?*c_void, &image)), "Unable to open image info.", @src());
    assert(boot_services.handleProtocol(image.?.device_handle.?, &uefi.protocols.SimpleFileSystemProtocol.guid, @ptrCast(*?*c_void, &fs)), "Unable to open filesystem", @src());
    assert(fs.?.openVolume(&esp), "Unable to open volume.", @src());
}

pub const LoadFileError = error{
    FileOpenFailure,
    FileInfoFailure,
    AllocationFailure,
    ReadFailure,
};

pub fn loadFile(path: [:0]const u16, open_mode: u64, attributes: u64) LoadFileError![]align(8) u8 {
    // Open file
    var file: *uefi.protocols.FileProtocol = undefined;
    switch (esp.open(&file, path, open_mode, attributes)) {
        .Success => {},
        else => |sts| {
            con_out_writer.print("Failed to open file handle: {}", .{sts}) catch unreachable;
            return LoadFileError.FileOpenFailure;
        },
    }

    // Get file size
    var info_buffer_size: usize = 0;
    var info_buffer: *uefi.protocols.FileInfo = undefined;
    _ = file.getInfo(&uefi.protocols.FileProtocol.guid, &info_buffer_size, @ptrCast([*]u8, info_buffer));
    assert(info_buffer_size > 0, "File too small", @src());
    switch (uefi.system_table.boot_services.?.allocatePool(uefi.tables.MemoryType.LoaderData, info_buffer_size, @ptrCast(*[*]align(8) u8, &info_buffer))) {
        .Success => {},
        else => |sts| {
            con_out_writer.print("Failed to allocate memory for file info: {}", .{sts}) catch unreachable;
            return LoadFileError.AllocationFailure;
        },
    }
    switch (file.getInfo(&uefi.protocols.FileProtocol.guid, &info_buffer_size, @ptrCast([*]u8, info_buffer))) {
        .Success => {},
        else => |sts| {
            con_out_writer.print("Failed to get file info: {}", .{sts}) catch unreachable;
            return LoadFileError.FileInfoFailure;
        },
    }

    // Read file data
    var buffer_size: usize = info_buffer.file_size;
    _ = uefi.system_table.boot_services.?.freePool(@ptrCast([*]align(8) u8, &info_buffer));
    var buffer: [*]align(8) u8 = undefined;
    switch (uefi.system_table.boot_services.?.allocatePool(uefi.tables.MemoryType.LoaderData, buffer_size, &buffer)) {
        .Success => {},
        else => |sts| {
            con_out_writer.print("Failed to allocate file buffer: {}", .{sts}) catch unreachable;
            return LoadFileError.AllocationFailure;
        },
    }
    switch (file.read(&buffer_size, buffer)) {
        .Success => {},
        else => |sts| {
            con_out_writer.print("Failed to read file data: {}", .{sts}) catch unreachable;
            return LoadFileError.ReadFailure;
        },
    }
    // Close file
    _ = file.close();

    return buffer[0..buffer_size];
}