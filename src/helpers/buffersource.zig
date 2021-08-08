const std = @import("std");

pub fn BufferSource(comptime BufferType: type) type {
    return struct {
        stream: std.io.FixedBufferStream(BufferType),

        pub fn init(buffer: BufferType) @This() {
            return .{ .stream = std.io.fixedBufferStream(buffer) };
        }

        pub fn getEndPos(self: *@This()) std.io.FixedBufferStream(BufferType).GetSeekPosError!u64 {
            return self.stream.getEndPos();
        }

        pub fn getPos(self: *@This()) std.io.FixedBufferStream(BufferType).GetSeekPosError!u64 {
            return self.stream.getPos();
        }

        pub fn read(self: *@This(), dest: []u8) std.io.FixedBufferStream(BufferType).ReadError!u64 {
            return self.stream.read(dest);
        }

        pub fn seekableStream(self: *@This()) std.io.FixedBufferStream(BufferType) {
            return self.stream;
        }

        pub fn reader(self: *@This()) std.io.Reader(*std.io.FixedBufferStream(BufferType), std.io.FixedBufferStream(BufferType).ReadError, std.io.FixedBufferStream(BufferType).read) {
            return self.stream.reader();
        }

        pub fn reset(self: *@This()) void {
            self.stream.reset();
        }

        pub fn seekBy(self: *@This(), amt: i64) std.io.FixedBufferStream(BufferType).SeekError!void {
            return self.stream.seekBy(amt);
        }

        pub fn seekTo(self: *@This(), pos: u64) std.io.FixedBufferStream(BufferType).SeekError!void {
            return self.stream.seekTo(pos);
        }
    };
}