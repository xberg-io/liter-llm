# LiterLlm

Universal LLM API client with Rust-powered polyglot bindings.

## Installation

Add to your `Package.swift`:

```swift
.package(url = "https://github.com/example/LiterLlm.git", branch: "main"),
```

## Building

```sh
cargo build -p liter-llm-swift
# Copy generated sources (see BUILDING.md for details)
swift build --package-path packages/swift
```

For detailed build instructions, see [BUILDING.md](BUILDING.md).

## License

MIT
