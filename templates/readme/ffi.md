# liter-llm - FFI (C/C++) Bindings

{% include 'partials/badges.html' %}

Universal LLM API client with Rust-powered polyglot bindings.

## What This Package Provides

- **Stable C ABI** — direct calls into the Rust client from C, C++, and secondary bindings.
- **Provider/model routing** — chat, streaming, embeddings, tools, search, OCR, and structured output use the same registry as every package.
- **Native integration point** — link the shared/static library when a language does not use a first-party binding.
- **Explicit ownership** — C callers own handles and buffers according to the generated header contract.

## Installation

Link against `libliter-llm_ffi` and include `liter_llm.h`.

See the build instructions in the main repository.

## Quick Start

```c
#include "liter_llm.h"

int main(void) {
    // See https://github.com/kreuzberg-dev/liter-llm for usage examples.
    return 0;
}
```

## Documentation

For full documentation, see the [liter-llm repository](https://github.com/kreuzberg-dev/liter-llm).

## License

See the [LICENSE](https://github.com/kreuzberg-dev/liter-llm/blob/main/LICENSE) file in the root repository.
