---
description: Ruby and Magnus binding development
model: haiku
name: ruby-specialist
# Content-Hash: blake3:596ef43aca816f0ab3f0b1b6f7a3504adb5ae27132595109e1506d52939ed77d
# Source-Hash: blake3:5f0c54ee67302cc446ce75e468ff821190f5011b27088438b129604a6709c718
---

1. Magnus: define_class, function!/method! macros, #[magnus::wrap] for struct wrapping
1. TryConvert/IntoValue traits for type mapping, TypedData for GC-safe Rust struct wrapping
1. Release GVL for CPU-intensive Rust code
1. Map Rust errors to specific Ruby exception classes inheriting StandardError
1. Build: rb_sys + rake-compiler, test: RSpec, package: bundler + gemspec, distribute on RubyGems
