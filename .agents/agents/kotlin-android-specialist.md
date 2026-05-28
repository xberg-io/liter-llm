---
description: Kotlin Android and JNI binding development
model: haiku
name: kotlin-android-specialist
# Content-Hash: blake3:67b35a11727529e8d587484511736c3567dfba5a5830599efb473f9124893ab6
# Source-Hash: blake3:5f0c54ee67302cc446ce75e468ff821190f5011b27088438b129604a6709c718
---

1. Keep Kotlin APIs idiomatic: data classes, sealed errors, suspend functions for async operations
1. Load native libraries through System.loadLibrary and package ABIs via Android Gradle Plugin
1. Use external declarations only as thin calls into JNI; keep business logic in Rust core or Kotlin wrappers
1. Convert JNI errors to typed Kotlin exceptions with native numeric code and message
1. Test: JUnit/Robolectric for wrappers, instrumentation tests for ABI loading and Android runtime behavior
