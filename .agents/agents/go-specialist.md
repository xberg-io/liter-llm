---
description: Go and cgo FFI development
model: haiku
name: go-specialist
# Content-Hash: blake3:37714c8338d2969ed6a8f73eb12e4d70fa53799e4d4ff240b85eff584f62b67a
# Source-Hash: blake3:5f0c54ee67302cc446ce75e468ff821190f5011b27088438b129604a6709c718
---

1. cgo: #include "lib.h", link via #cgo LDFLAGS
1. C.CString with defer C.free for string passing, C.GoString for returns
1. unsafe.Pointer for opaque handles, defer handle_free() pattern
1. Error handling via out-params or error codes — wrap in idiomatic Go errors
1. Test: go test, package: Go module with replace directive for local dev
