const Builder = @import("std").build.Builder;
const Target = @import("std").Target;

pub fn build(builder: *Builder) void {
    const fwlauncher = builder.addExecutable("BOOTX64", "src/main.zig");
    fwlauncher.setBuildMode(builder.standardReleaseOptions());
    fwlauncher.setTarget(.{
        .cpu_arch = Target.Cpu.Arch.x86_64,
        .os_tag = Target.Os.Tag.uefi,
        .abi = Target.Abi.msvc,
    });
    fwlauncher.setOutputDir("Build/Drive/EFI/BOOT");
    fwlauncher.addPackage(.{
        .name = "helpers",
        .path = .{ .path = "src/helpers/helpers.zig" },
    });
    builder.default_step.dependOn(&fwlauncher.step);
    builder.installArtifact(fwlauncher);
}