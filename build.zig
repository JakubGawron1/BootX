// Copyright (c) 2021 VisualDevelopment. All rights reserved.
const std = @import("std");
const deps = @import("./deps.zig");

pub fn build(builder: *std.build.Builder) void {
    const exe = builder.addExecutable("BOOTX64", "src/main.zig");
    exe.setTarget(.{
        .cpu_arch = std.Target.Cpu.Arch.x86_64,
        .os_tag = std.Target.Os.Tag.uefi,
        .abi = std.Target.Abi.msvc,
    });
    exe.setBuildMode(builder.standardReleaseOptions());
    exe.setOutputDir("Build/Drive/EFI/BOOT");
    deps.addAllTo(exe);
    exe.addPackage(.{
        .name = "helpers",
        .path = .{ .path = "src/helpers/helpers.zig" },
    });
    exe.install();
}
