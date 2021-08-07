// Copyright (c) 2021 VisualDevelopment. All rights reserved.
const std = @import("std");
const uefi = std.os.uefi;
const con_out_writer = @import("conoutwriter.zig").con_out_writer;

pub fn assert(expr: anytype, msg: [:0]const u8, srcLoc: std.builtin.SourceLocation) void {
    const isValid = switch (@TypeOf(expr)) {
        bool, uefi.Status => true,
        else => false,
    };

    if (!isValid) @compileError("assert only accepts expressions of type 'bool' and 'uefi.Status'");

    const isStatus = @TypeOf(expr) == uefi.Status;
    if (if (isStatus) expr != uefi.Status.Success else !expr) {
        con_out_writer.print("Assertion failed at {s}:{d}:{d}, {s}: {s}\n\r{s}{" ++ if (isStatus) "}" else "s}", .{
            std.fs.path.basename(srcLoc.file),
            srcLoc.line,
            srcLoc.column,
            srcLoc.fn_name,
            msg,
            if (isStatus) "Context: UEFI function called returned " else "",
            if (isStatus) expr else ""
        }) catch unreachable;

        while (true) {}
    }
}