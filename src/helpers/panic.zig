const builtin = @import("std").builtin;
const fmt = @import("std").fmt;
const utf8ToUtf16Le = @import("std").unicode.utf8ToUtf16Le;
const uefi = @import("std").os.uefi;
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