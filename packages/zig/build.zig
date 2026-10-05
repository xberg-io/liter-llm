const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // Default library/include search paths follow the conventional Cargo workspace
    // layout. `alef publish package --lang zig` rewrites this file for the
    // distributed tarball so consumers link the bundled lib/ and include/ dirs.
    // Override with -Dffi_path=... and -Dffi_include_path=... if your layout differs.
    const ffi_path_option = b.option([]const u8, "ffi_path", "Path to directory containing libliter_llm_ffi.{dylib,so,dll,a}") orelse "../../target/release";
    const ffi_path: std.Build.LazyPath = if (std.fs.path.isAbsolute(ffi_path_option))
        .{ .cwd_relative = ffi_path_option }
    else
        b.path(ffi_path_option);

    const ffi_include_option = b.option([]const u8, "ffi_include_path", "Path to directory containing the FFI C header") orelse "../../crates/liter-llm-ffi/include";
    const ffi_include: std.Build.LazyPath = if (std.fs.path.isAbsolute(ffi_include_option))
        .{ .cwd_relative = ffi_include_option }
    else
        b.path(ffi_include_option);

    const ffi_header = b.pathJoin(&.{ ffi_include_option, "liter_llm.h" });
    const translate_c = b.addTranslateC(.{
        .root_source_file = if (std.fs.path.isAbsolute(ffi_header))
            .{ .cwd_relative = ffi_header }
        else
            b.path(ffi_header),
        .target = target,
        .optimize = optimize,
    });
    translate_c.addIncludePath(ffi_include);

    const module = b.addModule("liter_llm", .{
        .root_source_file = b.path("src/liter_llm.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    module.addImport("c", translate_c.createModule());
    module.addLibraryPath(ffi_path);
    module.addIncludePath(ffi_include);
    module.linkSystemLibrary("liter_llm_ffi", .{});

    // `src/liter_llm.zig` is alef-generated and has no `test` blocks; the real
    // coverage lives in test/liter_llm_test.zig, which imports this module. ~keep
    const test_module = b.createModule(.{
        .root_source_file = b.path("test/liter_llm_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    test_module.addImport("liter_llm", module);
    test_module.addLibraryPath(ffi_path);
    test_module.addIncludePath(ffi_include);
    test_module.linkSystemLibrary("liter_llm_ffi", .{});

    const tests = b.addTest(.{
        .root_module = test_module,
    });

    const run_tests = b.addRunArtifact(tests);
    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_tests.step);

    const example_module = b.createModule(.{
        .root_source_file = b.path("examples/example.zig"),
        .target = target,
        .optimize = optimize,
    });
    const example_exe = b.addExecutable(.{
        .name = "example",
        .root_module = example_module,
    });
    const example_run = b.addRunArtifact(example_exe);
    const example_step = b.step("example", "Build and run examples/example.zig");
    example_step.dependOn(&example_run.step);
}
