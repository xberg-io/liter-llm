const std = @import("std");
const builtin = @import("builtin");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const test_step = b.step("test", "Run tests");

    // Fetch the published Zig package from the registry.
    const liter_llm_module = b.dependency("liter_llm", .{
        .target = target,
        .optimize = optimize,
    }).module("liter_llm");

    const _alloc = b.allocator;
    var mock_server_url: ?[]const u8 = b.graph.environ_map.get("MOCK_SERVER_URL");
    var mock_servers_json: ?[]const u8 = null;
    var mock_servers_map = std.StringHashMap([]const u8).init(_alloc);
    if (mock_server_url == null) {
        const _bin = b.pathFromRoot("../rust/target/release/mock-server");
        const _fixtures = b.pathFromRoot("../../fixtures");
        var _threaded = std.Io.Threaded.init(_alloc, .{});
        const _io = _threaded.io();
        const _spawned = std.process.spawn(_io, .{
            .argv = &.{ _bin, _fixtures },
            .stdin = .pipe,
            .stdout = .pipe,
            .stderr = .inherit,
        });
        if (_spawned) |_child| {
            // The child is intentionally not awaited: it lives for the duration
            // of the `zig build` process, which spans test execution.
            const _stdout = _child.stdout.?;
            var _buf: [65536]u8 = undefined;
            var _file_reader = _stdout.readerStreaming(_io, &_buf);
            const _r = &_file_reader.interface;
            // Read startup lines: MOCK_SERVER_URL= then MOCK_SERVERS= (always
            // emitted, possibly `{}`). Cap the loop so a misbehaving server
            // cannot block the build indefinitely.
            var _saw_url = false;
            var _i: usize = 0;
            while (_i < 64) : (_i += 1) {
                const _line_raw = _r.takeDelimiterExclusive('\n') catch break;
                const _line = std.mem.trim(u8, _line_raw, " \r\t");
                if (std.mem.startsWith(u8, _line, "MOCK_SERVER_URL=")) {
                    mock_server_url = _alloc.dupe(u8, _line["MOCK_SERVER_URL=".len..]) catch null;
                    _saw_url = true;
                } else if (std.mem.startsWith(u8, _line, "MOCK_SERVERS=")) {
                    const _json = _line["MOCK_SERVERS=".len..];
                    mock_servers_json = _alloc.dupe(u8, _json) catch null;
                    if (std.json.parseFromSlice(std.json.Value, _alloc, _json, .{})) |_parsed| {
                        if (_parsed.value == .object) {
                            var _entries = _parsed.value.object.iterator();
                            while (_entries.next()) |_entry| {
                                if (_entry.value_ptr.* == .string) {
                                    const _key = std.fmt.allocPrint(_alloc, "MOCK_SERVER_{s}", .{_entry.key_ptr.*}) catch continue;
                                    for (_key) |*_c| _c.* = std.ascii.toUpper(_c.*);
                                    const _val = _alloc.dupe(u8, _entry.value_ptr.*.string) catch continue;
                                    mock_servers_map.put(_key, _val) catch {};
                                }
                            }
                        }
                    } else |_| {}
                    break;
                } else if (_saw_url) {
                    break;
                }
            }
        } else |_| {
            // Binary not built — leave mock_server_url null so tests surface a
            // clear connection error rather than a build failure.
        }
    }

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
    const batches_run = b.addRunArtifact(batches_tests);
    if (mock_server_url) |_url| {
        batches_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        batches_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            batches_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const chat_run = b.addRunArtifact(chat_tests);
    if (mock_server_url) |_url| {
        chat_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        chat_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            chat_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const configuration_run = b.addRunArtifact(configuration_tests);
    if (mock_server_url) |_url| {
        configuration_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        configuration_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            configuration_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const contract_run = b.addRunArtifact(contract_tests);
    if (mock_server_url) |_url| {
        contract_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        contract_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            contract_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const custom_provider_run = b.addRunArtifact(custom_provider_tests);
    if (mock_server_url) |_url| {
        custom_provider_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        custom_provider_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            custom_provider_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const embed_run = b.addRunArtifact(embed_tests);
    if (mock_server_url) |_url| {
        embed_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        embed_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            embed_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const error_handling_run = b.addRunArtifact(error_handling_tests);
    if (mock_server_url) |_url| {
        error_handling_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        error_handling_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            error_handling_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const files_run = b.addRunArtifact(files_tests);
    if (mock_server_url) |_url| {
        files_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        files_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            files_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const image_generate_run = b.addRunArtifact(image_generate_tests);
    if (mock_server_url) |_url| {
        image_generate_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        image_generate_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            image_generate_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const list_models_run = b.addRunArtifact(list_models_tests);
    if (mock_server_url) |_url| {
        list_models_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        list_models_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            list_models_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const moderate_run = b.addRunArtifact(moderate_tests);
    if (mock_server_url) |_url| {
        moderate_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        moderate_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            moderate_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const ocr_run = b.addRunArtifact(ocr_tests);
    if (mock_server_url) |_url| {
        ocr_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        ocr_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            ocr_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const parity_run = b.addRunArtifact(parity_tests);
    if (mock_server_url) |_url| {
        parity_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        parity_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            parity_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const rerank_run = b.addRunArtifact(rerank_tests);
    if (mock_server_url) |_url| {
        rerank_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        rerank_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            rerank_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const responses_run = b.addRunArtifact(responses_tests);
    if (mock_server_url) |_url| {
        responses_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        responses_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            responses_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const search_run = b.addRunArtifact(search_tests);
    if (mock_server_url) |_url| {
        search_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        search_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            search_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const smoke_run = b.addRunArtifact(smoke_tests);
    if (mock_server_url) |_url| {
        smoke_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        smoke_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            smoke_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const speech_run = b.addRunArtifact(speech_tests);
    if (mock_server_url) |_url| {
        speech_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        speech_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            speech_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    test_step.dependOn(&speech_run.step);

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
    const tool_calling_run = b.addRunArtifact(tool_calling_tests);
    if (mock_server_url) |_url| {
        tool_calling_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        tool_calling_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            tool_calling_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const transcribe_run = b.addRunArtifact(transcribe_tests);
    if (mock_server_url) |_url| {
        transcribe_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        transcribe_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            transcribe_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
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
    const types_run = b.addRunArtifact(types_tests);
    if (mock_server_url) |_url| {
        types_run.setEnvironmentVariable("MOCK_SERVER_URL", _url);
    }
    if (mock_servers_json) |_json| {
        types_run.setEnvironmentVariable("MOCK_SERVERS", _json);
    }
    {
        var _it = mock_servers_map.iterator();
        while (_it.next()) |_entry| {
            types_run.setEnvironmentVariable(_entry.key_ptr.*, _entry.value_ptr.*);
        }
    }
    test_step.dependOn(&types_run.step);

}
