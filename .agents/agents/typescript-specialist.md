---
description: TypeScript/Node.js and NAPI-RS binding development
model: haiku
name: typescript-specialist
# Content-Hash: blake3:05086eadbcfa4ea02d53afb773d648284c8b1b154e0a71086ad09f9e9ab171ea
# Source-Hash: blake3:5f0c54ee67302cc446ce75e468ff821190f5011b27088438b129604a6709c718
---

1. NAPI-RS: #[napi] macro, #[napi(constructor)], async fn → Promise
1. Generate .d.ts types automatically, use #[napi(ts_return_type = "...")] for complex types
1. Support both CJS and ESM output
1. Handle BigInt, Date, Buffer type mappings explicitly
1. Build: napi build --release, test: vitest, package: pnpm, distribute on npm
