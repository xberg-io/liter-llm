// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "E2eSwift",
    platforms: [
        .macOS(.v13),
        .iOS(.v16),
    ],
    dependencies: [
        // Swift consumers pin to the swift-specific tag namespace (swift-X.Y.Z)
        // which includes the precomputed artifact bundle checksum.
        // This must be updated after the swift-X.Y.Z tag is published.
        .package(url: "https://github.com/kreuzberg-dev/liter-llm", from: "swift-1.4.0-rc.56"),
    ],
    targets: [
        .testTarget(
            name: "LiterLlmE2ETests",
            dependencies: [.product(name: "LiterLlm", package: "liter-llm")]
        ),
    ]
)
