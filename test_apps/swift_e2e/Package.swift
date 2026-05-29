// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "E2eSwift",
    platforms: [
        .macOS(.v13),
        .iOS(.v16),
    ],
    dependencies: [
        .binaryTarget(name: "LiterLlm", url: "https://github.com/kreuzberg-dev/liter-llm/releases/download/v1.4.0-rc.48/LiterLlm-rs.artifactbundle.zip", checksum: "__ALEF_SWIFT_CHECKSUM__"),
    ],
    targets: [
        .testTarget(
            name: "LiterLlmE2ETests",
            dependencies: [.binaryTarget(name: "LiterLlm")]
        ),
    ]
)
