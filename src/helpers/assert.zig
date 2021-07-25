const uefi = @import("std").os.uefi;
const utf8ToUtf16Le = @import("std").unicode.utf8ToUtf16Le;
const fmt = @import("std").fmt;
const builtin = @import("std").builtin;
const path = @import("std").fs.path;
const stringToEnum = @import("std").meta.stringToEnum;

fn assertValidExpr(comptime T: anytype) void {
    const isValid = switch (T) {
        bool, uefi.Status => true,
        else => false,
    };

    if (!isValid) @compileError("assert only accepts expressions of type 'bool' and 'uefi.Status'");
}

pub fn assert(expr: anytype, msg: [:0]const u8, srcLoc: builtin.SourceLocation) void {
    assertValidExpr(@TypeOf(expr));
    const ok = if (@TypeOf(expr) == bool) expr else expr == uefi.Status.Success;
    if (!ok) {
        var buf = [_:0]u8{0} ** 512;
        var buf16 = [_:0]u16{0} ** 512;

        _ = fmt.bufPrintZ(buf[0..], "Assertion failed at {s}:{d}:{d}, {s}: {s}\n\r{s}{" ++ if (@TypeOf(expr) == bool) "s}" else "}", .{
            path.basename(srcLoc.file),
            srcLoc.line,
            srcLoc.column,
            srcLoc.fn_name,
            msg,
            if (@TypeOf(expr) == uefi.Status) "Context: UEFI function called returned " else "",
            if (@TypeOf(expr) == uefi.Status) expr else ""
        }) catch unreachable;
        _ = utf8ToUtf16Le(buf16[0..], buf[0..]) catch unreachable;
        _ = uefi.system_table.con_out.?.outputString(buf16[0..]);
        while (true) {}
    }
}