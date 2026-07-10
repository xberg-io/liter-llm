// swift-tools-version: 6.0
import PackageDescription
import Foundation

// NOTE: Run `cargo build -p liter-llm-swift` and then rerun `alef generate`

let rustTargetDir = (#filePath as NSString).deletingLastPathComponent.appending("/../../target")

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
    .target(
      name: "RustBridgeC",
      path: "Sources/RustBridgeC",
      publicHeadersPath: "."
    ),
    .target(
      name: "RustBridge",
      dependencies: ["RustBridgeC"],
      path: "Sources/RustBridge",
      linkerSettings: [
        .unsafeFlags([
          "-L\(rustTargetDir)/release",
          "-L\(rustTargetDir)/debug",
          "-Xlinker", "-rpath", "-Xlinker", "\(rustTargetDir)/release",
          "-Xlinker", "-rpath", "-Xlinker", "\(rustTargetDir)/debug",
        ]),
        .linkedLibrary("liter_llm_swift"),
        .linkedLibrary("liter_llm_ffi"),
        .linkedFramework("Security", .when(platforms: [.macOS, .iOS])),
        .linkedFramework("CoreFoundation", .when(platforms: [.macOS, .iOS])),
        .linkedFramework("SystemConfiguration", .when(platforms: [.macOS])),
      ]
    ),
    .target(
      name: "LiterLlm", dependencies: ["RustBridge"],
      path: "Sources/LiterLlm",
      exclude: ["LICENSE"]),
    .testTarget(
      name: "LiterLlmTests", dependencies: ["LiterLlm"],
      path: "Tests/LiterLlmTests"),
  ]
)
