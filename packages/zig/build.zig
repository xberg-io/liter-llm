const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const ffi_path = b.option(
        []const u8,
        "ffi_path",
        "Path to directory containing libliter_llm_ffi.{dylib,so,dll,a}"
    ) orelse "../../target/release";

    const ffi_include = b.option(
        []const u8,
        "ffi_include_path",
        "Path to directory containing the FFI C header"
    ) orelse "../../crates/liter-llm-ffi/include";

    const module = b.addModule("liter_llm", .{
        .root_source_file = b.path("src/liter_llm.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    module.addLibraryPath(.{ .cwd_relative = ffi_path });
    module.addIncludePath(.{ .cwd_relative = ffi_include });
    module.linkSystemLibrary("liter_llm_ffi", .{});

    const test_module = b.createModule(.{
        .root_source_file = b.path("src/liter_llm.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    test_module.addLibraryPath(.{ .cwd_relative = ffi_path });
    test_module.addIncludePath(.{ .cwd_relative = ffi_include });
    test_module.linkSystemLibrary("liter_llm_ffi", .{});

    const tests = b.addTest(.{
        .root_module = test_module,
    });

    const run_tests = b.addRunArtifact(tests);
    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_tests.step);
}
