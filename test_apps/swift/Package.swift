// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "E2eSwift",
    platforms: [
        .macOS(.v13),
        .iOS(.v14),
    ],
    dependencies: [
        .package(url: "https://github.com/kreuzberg-dev/liter-llm.git", from: "1.4.0-rc.47"),
    ],
    targets: [
        .testTarget(
            name: "LiterLlmTests",
            dependencies: [.product(name: "LiterLlm", package: "liter-llm")]
        ),
    ]
)
