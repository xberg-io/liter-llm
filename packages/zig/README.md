# liter-llm - Zig Bindings

Universal LLM API client with Rust-powered polyglot bindings.

## Installation

Add to `build.zig.zon`:

```zig
.dependencies = .{
    .liter_llm = .{ .url = "<tarball-url>" },
};
```

## Quick Start

```zig
const liter_llm = @import("liter_llm");

// Call generated wrapper functions; strings allocated by the FFI must
// be released with `liter_llm._free_string`.
```

## Documentation

For full documentation, see the [liter-llm repository](https://github.com/kreuzberg-dev/liter-llm).

## Part of Kreuzberg, Inc

- [Kreuzberg](https://docs.kreuzberg.dev) — document intelligence: text, tables, metadata from 91+ formats with optional OCR.
- [Kreuzberg Cloud](https://docs.kreuzberg.cloud) — managed extraction API with SDKs, dashboards, and observability.
- [kreuzcrawl](https://docs.kreuzcrawl.kreuzberg.dev) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://docs.html-to-markdown.kreuzberg.dev) — fast, lossless HTML→Markdown engine.
- [tree-sitter-language-pack](https://docs.tree-sitter-language-pack.kreuzberg.dev) — tree-sitter grammars and code-intelligence primitives.
- [Discord](https://discord.gg/xt9WY3GnKR) — community, roadmap, announcements.

## License

See the [LICENSE](https://github.com/kreuzberg-dev/liter-llm/blob/main/LICENSE) file in the root repository.
