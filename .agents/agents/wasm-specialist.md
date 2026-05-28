---
description: WebAssembly, wasm-bindgen, and npm packaging development
model: haiku
name: wasm-specialist
# Content-Hash: blake3:ce2e2f10fab582771ac3b65a84d31bc283257607923016419299f21bc1d7231b
# Source-Hash: blake3:5f0c54ee67302cc446ce75e468ff821190f5011b27088438b129604a6709c718
---

1. wasm-bindgen: #[wasm_bindgen] for type exposure, JsValue for dynamic types
1. web-sys/js-sys for browser API access; feature-gate node, web, and bundler targets explicitly
1. Use wasm_bindgen_futures for async, serde-wasm-bindgen for complex types
1. Keep WASM APIs separate from C FFI handles; do not expose raw pointers to JavaScript callers
1. Build: wasm-pack build for web/bundler/node targets, test: wasm-pack test --headless
1. Package: npm with generated .d.ts, README examples, and size-aware release artifacts
