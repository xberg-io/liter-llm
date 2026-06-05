// swift-tools-version: 6.0
// Root-level Package.swift — alef-generated for published distributions.
//
// This manifest uses `.binaryTarget` for pre-built XCFramework/artifact bundles.
// External consumers depend on this via `.package(url: "...", from: "...")`.
//
// For in-tree development, see `packages/swift/Package.swift` and
// `packages/swift/README.md` for the source-based workflow.
import PackageDescription

let package = Package(
  name: "LiterLlm",
  platforms: [
    .macOS(.v13),
    .iOS(.v16),
  ],
  products: [
    .library(name: "LiterLlm", targets: ["LiterLlm"])
  ],
  targets: [
    // RustBridge: pre-built binary target containing the compiled Rust library
    // for macOS (arm64, x86_64), iOS (device, simulator), and Linux (arm64, x86_64).
    // The binary includes C headers for swift-bridge interop.
    .binaryTarget(
      name: "RustBridge",
      url: "https://github.com/kreuzberg-dev/liter-llm/releases/download/v1.4.0-rc.61/LiterLlm-rs.artifactbundle.zip",
      checksum: "01c746da66f796f9b5b5a02091f967433d550cdf08ed01bed31e35edf06c9a8a"
    ),
    .target(
      name: "LiterLlm",
      dependencies: ["RustBridge"],
      path: "packages/swift/Sources/LiterLlm"
    ),
  ]
)
