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

## License

See the [LICENSE](https://github.com/kreuzberg-dev/liter-llm/blob/main/LICENSE) file in the root repository.
