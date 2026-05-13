# Building LiterLlm

The Swift package wraps a Rust library via [swift-bridge](https://github.com/chinedufn/swift-bridge).
SwiftPM cannot invoke Cargo directly, so you must run the cargo build step first.

## Workflow

### 1. Build the Rust binding crate

From the **repository root**:

```sh
cargo build -p liter-llm-swift
```

This compiles `target/debug/libliter_llm_swift.a` and runs
`swift-bridge-build` in `build.rs`, which writes generated Swift and C sources
into `target/debug/build/liter-llm-swift-*/out/`.

### 2. Copy generated sources into the SwiftPM targets

The package uses two internal targets:
- `Sources/RustBridgeC/` — pure C target with the combined C header
- `Sources/RustBridge/`  — Swift bridge files that `import RustBridgeC`

```sh
OUT=$(ls -dt target/debug/build/liter-llm-swift-*/out 2>/dev/null | head -1)

# Combine C headers into the RustBridgeC target
cat "$OUT/SwiftBridgeCore.h" "$OUT/liter-llm-swift/liter-llm-swift.h" \
    > packages/swift/Sources/RustBridgeC/RustBridgeC.h

# Copy Swift bridge files, prepending "import RustBridgeC" so they see the C types.
# Use `{ echo ...; cat ...; }` rather than `printf "...$(cat)..."` because printf
# interprets `%` and `\` sequences in its format string, which would corrupt the
# generated Swift sources.
{ echo "import RustBridgeC"; cat "$OUT/SwiftBridgeCore.swift"; } \
    > packages/swift/Sources/RustBridge/SwiftBridgeCore.swift
{ echo "import RustBridgeC"; cat "$OUT/liter-llm-swift/liter-llm-swift.swift"; } \
    > packages/swift/Sources/RustBridge/liter-llm-swift.swift
```

If the glob `liter-llm-swift-*/out` matches multiple directories, `ls -dt ... | head -1`
picks the most recently modified one.

### 3. Build and test the Swift package

```sh
swift build --package-path packages/swift
swift test --package-path packages/swift
```

## Release builds

Replace `target/debug` with `target/release` and pass
`--configuration release` to `swift build`:

```sh
cargo build --release -p liter-llm-swift
OUT=$(ls -dt target/release/build/liter-llm-swift-*/out 2>/dev/null | head -1)

cat "$OUT/SwiftBridgeCore.h" "$OUT/liter-llm-swift/liter-llm-swift.h" \
    > packages/swift/Sources/RustBridgeC/RustBridgeC.h
{ echo "import RustBridgeC"; cat "$OUT/SwiftBridgeCore.swift"; } \
    > packages/swift/Sources/RustBridge/SwiftBridgeCore.swift
{ echo "import RustBridgeC"; cat "$OUT/liter-llm-swift/liter-llm-swift.swift"; } \
    > packages/swift/Sources/RustBridge/liter-llm-swift.swift

swift build --package-path packages/swift --configuration release
```

## Notes

- Files in `Sources/RustBridgeC/` and the generated Swift files in
  `Sources/RustBridge/` are **generated artifacts** — overwritten by the copy step.
- `Sources/RustBridge/RustBridge.swift` is a placeholder and is overwritten.
- `target/` is in `.gitignore`; regenerate after every `cargo clean`.

## Toolchain / SDK alignment

The Swift package and its E2E suite link against the host macOS SDK and pick up
the Swift toolchain that `xcode-select` resolves to. When the active toolchain
and the active SDK come from different Xcode majors, `swift build` / `swift test`
can fail at link time with `ld: framework 'SwiftUICore' not found` (or similar
missing-framework errors against `swiftCompatibility*` libraries). LiterLlm
itself does not depend on SwiftUI/SwiftUICore — these come in transitively
when the linker is asked to satisfy a stdlib referenced by a mismatched SDK.

### Known-good configuration

- Xcode and its bundled toolchain match (do **not** select a sideloaded
  swift.org toolchain via `TOOLCHAINS=…` unless its version matches the Xcode
  SDK exactly).
- `xcode-select -p` points at a single Xcode install whose `Contents/Developer/Toolchains/XcodeDefault.xctoolchain` ships the Swift used by `swift --version`.

Verify with:

```sh
xcode-select -p
xcodebuild -version
swift --version
xcrun --show-sdk-path --sdk macosx
```

The four outputs should agree on the major version (e.g. all pointing at
Xcode 26.x with the matching MacOSX26.x.sdk).

### If `swift test` fails with a missing-framework linker error

1. Unset any `TOOLCHAINS` env var.
2. Switch the active developer dir to the Xcode that owns the SDK you want:
   `sudo xcode-select -s /Applications/Xcode.app`.
3. Clean SwiftPM caches: `swift package --package-path packages/swift clean`.
4. Re-run from a fresh shell to drop stale environment.

Upstream context:

- Swift forums — Xcode 26 toolchain/SDK mismatches: <https://forums.swift.org/t/xcode-26-4-beta-1-swift-version-mismatch/84807>
- Swift forums — sideloaded toolchain SDK incompatibility: <https://forums.swift.org/t/unable-to-use-swiftui-with-latest-swift-6-0-1-compiler-due-to-toolchain-sdk-mismatch/74972>
- `swiftCompatibility*` removal symptom (Tauri tracker): <https://github.com/tauri-apps/tauri/issues/15066>
