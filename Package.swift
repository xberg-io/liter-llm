// swift-tools-version: 6.0
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
    .target(
      name: "RustBridgeC",
      path: "packages/swift/Sources/RustBridgeC",
      publicHeadersPath: "."
    ),
    .target(
      name: "RustBridge",
      dependencies: ["RustBridgeC"],
      path: "packages/swift/Sources/RustBridge",
      linkerSettings: [
        .unsafeFlags([
          "-Ltarget/release",
          "-Ltarget/debug",
        ]),
        .linkedLibrary("liter_llm_swift"),
        .linkedFramework("Security", .when(platforms: [.macOS, .iOS])),
        .linkedFramework("CoreFoundation", .when(platforms: [.macOS, .iOS])),
        .linkedFramework("SystemConfiguration", .when(platforms: [.macOS])),
      ]
    ),
    .target(
      name: "LiterLlm",
      dependencies: ["RustBridge"],
      path: "packages/swift/Sources/LiterLlm"
    ),
  ]
)
