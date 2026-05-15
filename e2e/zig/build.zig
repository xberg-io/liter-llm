const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const test_step = b.step("test", "Run tests");
    const ffi_path = b.option([]const u8, "ffi_path", "Path to directory containing libliter_llm_ffi") orelse "../../target/debug";
    const ffi_include = b.option([]const u8, "ffi_include_path", "Path to directory containing FFI header") orelse "../../crates/liter-llm-ffi/include";

    const liter_llm_module = b.addModule("liter_llm", .{
        .root_source_file = b.path("../../packages/zig/src/liter_llm.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    liter_llm_module.addLibraryPath(.{ .cwd_relative = ffi_path });
    liter_llm_module.addIncludePath(.{ .cwd_relative = ffi_include });
    liter_llm_module.linkSystemLibrary("liter_llm_ffi", .{});

    const batches_module = b.createModule(.{
        .root_source_file = b.path("src/batches_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    batches_module.addImport("liter_llm", liter_llm_module);
    const batches_tests = b.addTest(.{
        .name = "batches_test",
        .root_module = batches_module,
        .use_llvm = true,
    });
    b.installArtifact(batches_tests);
    const batches_run = b.addRunArtifact(batches_tests);
    batches_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&batches_run.step);

    const chat_module = b.createModule(.{
        .root_source_file = b.path("src/chat_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    chat_module.addImport("liter_llm", liter_llm_module);
    const chat_tests = b.addTest(.{
        .name = "chat_test",
        .root_module = chat_module,
        .use_llvm = true,
    });
    b.installArtifact(chat_tests);
    const chat_run = b.addRunArtifact(chat_tests);
    chat_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&chat_run.step);

    const configuration_module = b.createModule(.{
        .root_source_file = b.path("src/configuration_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    configuration_module.addImport("liter_llm", liter_llm_module);
    const configuration_tests = b.addTest(.{
        .name = "configuration_test",
        .root_module = configuration_module,
        .use_llvm = true,
    });
    b.installArtifact(configuration_tests);
    const configuration_run = b.addRunArtifact(configuration_tests);
    configuration_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&configuration_run.step);

    const contract_module = b.createModule(.{
        .root_source_file = b.path("src/contract_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    contract_module.addImport("liter_llm", liter_llm_module);
    const contract_tests = b.addTest(.{
        .name = "contract_test",
        .root_module = contract_module,
        .use_llvm = true,
    });
    b.installArtifact(contract_tests);
    const contract_run = b.addRunArtifact(contract_tests);
    contract_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&contract_run.step);

    const custom_provider_module = b.createModule(.{
        .root_source_file = b.path("src/custom_provider_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    custom_provider_module.addImport("liter_llm", liter_llm_module);
    const custom_provider_tests = b.addTest(.{
        .name = "custom_provider_test",
        .root_module = custom_provider_module,
        .use_llvm = true,
    });
    b.installArtifact(custom_provider_tests);
    const custom_provider_run = b.addRunArtifact(custom_provider_tests);
    custom_provider_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&custom_provider_run.step);

    const embed_module = b.createModule(.{
        .root_source_file = b.path("src/embed_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    embed_module.addImport("liter_llm", liter_llm_module);
    const embed_tests = b.addTest(.{
        .name = "embed_test",
        .root_module = embed_module,
        .use_llvm = true,
    });
    b.installArtifact(embed_tests);
    const embed_run = b.addRunArtifact(embed_tests);
    embed_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&embed_run.step);

    const error_handling_module = b.createModule(.{
        .root_source_file = b.path("src/error_handling_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    error_handling_module.addImport("liter_llm", liter_llm_module);
    const error_handling_tests = b.addTest(.{
        .name = "error_handling_test",
        .root_module = error_handling_module,
        .use_llvm = true,
    });
    b.installArtifact(error_handling_tests);
    const error_handling_run = b.addRunArtifact(error_handling_tests);
    error_handling_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&error_handling_run.step);

    const files_module = b.createModule(.{
        .root_source_file = b.path("src/files_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    files_module.addImport("liter_llm", liter_llm_module);
    const files_tests = b.addTest(.{
        .name = "files_test",
        .root_module = files_module,
        .use_llvm = true,
    });
    b.installArtifact(files_tests);
    const files_run = b.addRunArtifact(files_tests);
    files_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&files_run.step);

    const image_generate_module = b.createModule(.{
        .root_source_file = b.path("src/image_generate_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    image_generate_module.addImport("liter_llm", liter_llm_module);
    const image_generate_tests = b.addTest(.{
        .name = "image_generate_test",
        .root_module = image_generate_module,
        .use_llvm = true,
    });
    b.installArtifact(image_generate_tests);
    const image_generate_run = b.addRunArtifact(image_generate_tests);
    image_generate_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&image_generate_run.step);

    const list_models_module = b.createModule(.{
        .root_source_file = b.path("src/list_models_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    list_models_module.addImport("liter_llm", liter_llm_module);
    const list_models_tests = b.addTest(.{
        .name = "list_models_test",
        .root_module = list_models_module,
        .use_llvm = true,
    });
    b.installArtifact(list_models_tests);
    const list_models_run = b.addRunArtifact(list_models_tests);
    list_models_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&list_models_run.step);

    const moderate_module = b.createModule(.{
        .root_source_file = b.path("src/moderate_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    moderate_module.addImport("liter_llm", liter_llm_module);
    const moderate_tests = b.addTest(.{
        .name = "moderate_test",
        .root_module = moderate_module,
        .use_llvm = true,
    });
    b.installArtifact(moderate_tests);
    const moderate_run = b.addRunArtifact(moderate_tests);
    moderate_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&moderate_run.step);

    const ocr_module = b.createModule(.{
        .root_source_file = b.path("src/ocr_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    ocr_module.addImport("liter_llm", liter_llm_module);
    const ocr_tests = b.addTest(.{
        .name = "ocr_test",
        .root_module = ocr_module,
        .use_llvm = true,
    });
    b.installArtifact(ocr_tests);
    const ocr_run = b.addRunArtifact(ocr_tests);
    ocr_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&ocr_run.step);

    const parity_module = b.createModule(.{
        .root_source_file = b.path("src/parity_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    parity_module.addImport("liter_llm", liter_llm_module);
    const parity_tests = b.addTest(.{
        .name = "parity_test",
        .root_module = parity_module,
        .use_llvm = true,
    });
    b.installArtifact(parity_tests);
    const parity_run = b.addRunArtifact(parity_tests);
    parity_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&parity_run.step);

    const rerank_module = b.createModule(.{
        .root_source_file = b.path("src/rerank_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    rerank_module.addImport("liter_llm", liter_llm_module);
    const rerank_tests = b.addTest(.{
        .name = "rerank_test",
        .root_module = rerank_module,
        .use_llvm = true,
    });
    b.installArtifact(rerank_tests);
    const rerank_run = b.addRunArtifact(rerank_tests);
    rerank_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&rerank_run.step);

    const responses_module = b.createModule(.{
        .root_source_file = b.path("src/responses_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    responses_module.addImport("liter_llm", liter_llm_module);
    const responses_tests = b.addTest(.{
        .name = "responses_test",
        .root_module = responses_module,
        .use_llvm = true,
    });
    b.installArtifact(responses_tests);
    const responses_run = b.addRunArtifact(responses_tests);
    responses_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&responses_run.step);

    const search_module = b.createModule(.{
        .root_source_file = b.path("src/search_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    search_module.addImport("liter_llm", liter_llm_module);
    const search_tests = b.addTest(.{
        .name = "search_test",
        .root_module = search_module,
        .use_llvm = true,
    });
    b.installArtifact(search_tests);
    const search_run = b.addRunArtifact(search_tests);
    search_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&search_run.step);

    const smoke_module = b.createModule(.{
        .root_source_file = b.path("src/smoke_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    smoke_module.addImport("liter_llm", liter_llm_module);
    const smoke_tests = b.addTest(.{
        .name = "smoke_test",
        .root_module = smoke_module,
        .use_llvm = true,
    });
    b.installArtifact(smoke_tests);
    const smoke_run = b.addRunArtifact(smoke_tests);
    smoke_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&smoke_run.step);

    const speech_module = b.createModule(.{
        .root_source_file = b.path("src/speech_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    speech_module.addImport("liter_llm", liter_llm_module);
    const speech_tests = b.addTest(.{
        .name = "speech_test",
        .root_module = speech_module,
        .use_llvm = true,
    });
    b.installArtifact(speech_tests);
    const speech_run = b.addRunArtifact(speech_tests);
    speech_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&speech_run.step);

    const streaming_module = b.createModule(.{
        .root_source_file = b.path("src/streaming_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    streaming_module.addImport("liter_llm", liter_llm_module);
    const streaming_tests = b.addTest(.{
        .name = "streaming_test",
        .root_module = streaming_module,
        .use_llvm = true,
    });
    b.installArtifact(streaming_tests);
    const streaming_run = b.addRunArtifact(streaming_tests);
    streaming_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&streaming_run.step);

    const tool_calling_module = b.createModule(.{
        .root_source_file = b.path("src/tool_calling_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    tool_calling_module.addImport("liter_llm", liter_llm_module);
    const tool_calling_tests = b.addTest(.{
        .name = "tool_calling_test",
        .root_module = tool_calling_module,
        .use_llvm = true,
    });
    b.installArtifact(tool_calling_tests);
    const tool_calling_run = b.addRunArtifact(tool_calling_tests);
    tool_calling_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&tool_calling_run.step);

    const transcribe_module = b.createModule(.{
        .root_source_file = b.path("src/transcribe_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    transcribe_module.addImport("liter_llm", liter_llm_module);
    const transcribe_tests = b.addTest(.{
        .name = "transcribe_test",
        .root_module = transcribe_module,
        .use_llvm = true,
    });
    b.installArtifact(transcribe_tests);
    const transcribe_run = b.addRunArtifact(transcribe_tests);
    transcribe_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&transcribe_run.step);

    const types_module = b.createModule(.{
        .root_source_file = b.path("src/types_test.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    types_module.addImport("liter_llm", liter_llm_module);
    const types_tests = b.addTest(.{
        .name = "types_test",
        .root_module = types_module,
        .use_llvm = true,
    });
    b.installArtifact(types_tests);
    const types_run = b.addRunArtifact(types_tests);
    types_run.setCwd(b.path("../../test_documents"));
    test_step.dependOn(&types_run.step);

}
