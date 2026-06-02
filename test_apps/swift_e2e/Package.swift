// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "E2eSwift",
    platforms: [
        .macOS(.v13),
        .iOS(.v16),
    ],
    targets: [
                .binaryTarget(name: "LiterLlm", url: "https://github.com/kreuzberg-dev/liter-llm/releases/download/v1.4.0-rc.48/LiterLlm-rs.artifactbundle.zip", checksum: "__ALEF_SWIFT_CHECKSUM__"),
        .testTarget(
            name: "LiterLlmE2ETests",
            dependencies: [.target(name: "LiterLlm")]
        ),
    ]
)
