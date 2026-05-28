---
name: add-language-generator
description: "Add a new language to Alef-based e2e test generation"
# Content-Hash: blake3:39d393afaf99570a1aadf554643bf240f79f78bc165ae1f33593f794f5df0af0
# Source-Hash: blake3:5f0c54ee67302cc446ce75e468ff821190f5011b27088438b129604a6709c718
---

# Add Language Generator

E2E test generation is handled by [Alef](https://github.com/kreuzberg-dev/alef). To add a new language:

## Steps

1. **Add language to `alef.toml`**: Configure the new language under `[e2e.languages.<lang>]` with appropriate test framework, file extensions, and assertion patterns.
1. **Add call overrides**: If the language needs special call syntax, add `[e2e.call.<call_name>.languages.<lang>]` overrides.
1. **Generate and test**: Run `task e2e:generate:all` then `task e2e:test:all` to verify the generated tests compile and pass.
1. **Update CI matrix**: Add the new language to the e2e test CI workflow.

## Checklist

- [ ] Language configured in `alef.toml` `[e2e]` section
- [ ] Generated tests use language-idiomatic assertion patterns
- [ ] Skip conditions translate to language-native skip annotations
- [ ] CI runs generated tests on all supported platforms
