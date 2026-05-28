---
description: Swift and C FFI binding development
model: haiku
name: swift-specialist
# Content-Hash: blake3:cc02a2f600b9d79d43c80a1834f8e2af85304b350e1d9bad10be3f47692e498b
# Source-Hash: blake3:5f0c54ee67302cc446ce75e468ff821190f5011b27088438b129604a6709c718
---

1. Import generated C headers through a module map or SwiftPM system library target
1. Wrap opaque pointers in final classes with deinit cleanup and explicit close() for deterministic release
1. Convert C strings via String(cString:) and document ownership for every returned buffer
1. Map C error codes to Swift Error types with numeric code, message, and failing operation
1. Test: XCTest, package: SwiftPM with binary artifacts for released native libraries
