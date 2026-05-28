---
description: Java, Panama FFM, and JNI binding development
model: haiku
name: java-specialist
# Content-Hash: blake3:de748d6685ee2e1593e470e0bd7da4b264ae4959ce60b2548165b76c68377e38
# Source-Hash: blake3:5f0c54ee67302cc446ce75e468ff821190f5011b27088438b129604a6709c718
---

1. Panama FFM (Java 21+): Linker.downcallHandle + FunctionDescriptor for native calls
1. MemorySegment for pointer types, Arena for lifecycle management
1. Map C error codes to typed Java exceptions with context preservation
1. String handling: MemorySegment.reinterpret() + getString() for C strings
1. Use JNI only for Android, callbacks, or APIs Panama cannot express cleanly
1. Test: JUnit 5 across OS/architecture classifiers, package: Maven Central
