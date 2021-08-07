// Copyright (c) 2021 VisualDevelopment. All rights reserved.
const std = @import("std");
const uefi = std.os.uefi;

pub const FailedToWrite = error{FailedToWrite};

fn writeFn(context: void, bytes: []const u8) FailedToWrite!usize {
    _ = context;
    const con_out = uefi.system_table.con_out.?;

    var str: [2:0]u16 = [_:0]u16{0} ** 2;
    for (bytes) |ch| {
        str[0] = ch;
        switch (con_out.outputString(@ptrCast(* const [1:0]u16, &str))) {
            uefi.Status.Success => {},
            else => return FailedToWrite.FailedToWrite,
        }
    }

    return bytes.len;
}

pub const con_out_writer = std.io.Writer(void, FailedToWrite, writeFn){.context = undefined};