// Copyright (c) 2021 VisualDevelopment. All rights reserved.
const std = @import("std");
const uefi = std.os.uefi;
const con_out_writer = @import("conoutwriter.zig").con_out_writer;

pub fn panic(msg: []const u8, stack_trace: ?*std.builtin.StackTrace) noreturn {
    _ = stack_trace;
    con_out_writer.print("Panic: {s}\n\rStack trace:\n\r\t0x{X}\n\r", .{ msg, @returnAddress() }) catch unreachable;
    var iter = std.debug.StackIterator.init(@returnAddress(), null);
    while (iter.next()) |addr| con_out_writer.print("\t0x{X}\n\r", .{addr}) catch unreachable;
    
    while (true) {}
}