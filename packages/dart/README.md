# liter_llm

Universal LLM API client with Rust-powered polyglot bindings.

## Installation

Add to your `pubspec.yaml`:

```yaml
dependencies:
  liter_llm: ^1.4.0-rc.27
```

Then run:

```sh
dart pub get
```

## Building

From the repository root:

```sh
cargo build -p liter-llm-dart
flutter_rust_bridge_codegen generate
dart pub get
dart analyze
dart test
```

For detailed build instructions, see [BUILDING.md](BUILDING.md).

## License

MIT
