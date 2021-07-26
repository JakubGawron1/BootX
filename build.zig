const Builder = @import("std").build.Builder;
const Target = @import("std").Target;

pub fn build(builder: *Builder) void {
    const exe = builder.addExecutable("BOOTX64", "src/main.zig");
    exe.setTarget(.{
        .cpu_arch = Target.Cpu.Arch.x86_64,
        .os_tag = Target.Os.Tag.uefi,
        .abi = Target.Abi.msvc,
    });
    exe.setBuildMode(builder.standardReleaseOptions());
    exe.setOutputDir("Build/Drive/EFI/BOOT");
    exe.addPackage(.{
        .name = "helpers",
        .path = .{ .path = "src/helpers/helpers.zig" },
    });
    exe.install();
}