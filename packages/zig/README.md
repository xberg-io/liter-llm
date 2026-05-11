# liter_llm

Universal LLM API client with Rust-powered polyglot bindings.

## Installation

Install Zig from [ziglang.org](https://ziglang.org/download/).

## Building

```sh
zig build
zig build test
```

## Usage

Add to your `build.zig.zon`:

```
.dependencies = .{
    .liter_llm = .{
        .path = "path/to/liter_llm",
    },
},
```

## License

MIT
