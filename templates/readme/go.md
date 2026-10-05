# liter-llm — Go

{% include 'partials/badges.html' %}
{% include 'partials/banner.html' %}
{% include 'partials/discord.html' %}

Universal LLM API client for Go. Access 165 LLM providers through a single interface backed by the Rust core.

> **Version {{ version }}**
> Report issues at [github.com/xberg-io/liter-llm](https://github.com/xberg-io/liter-llm/issues).

## What This Package Provides

- **Go module over the Rust client** — context-aware chat, streaming, embeddings, tool calls, search, and OCR.
- **Provider/model routing** — call `provider/model` names without provider-specific client branches.
- **Static-link workflow** — build against `liter-llm-ffi` and ship a self-contained Go binary.
- **Cross-binding parity** — behavior matches the Rust, Python, Node.js, Java, .NET, Ruby, PHP, Elixir, Swift, Dart, Zig, WASM, and C FFI packages.

## Install

### Using Go Modules

```bash
go get {{ package_name }}@latest
```

You'll need the native FFI library at build time. See [Building with Static Libraries](#building-with-static-libraries) below.

### Quick Start (Monorepo Development)

For development in the liter-llm monorepo:

```bash
# Build the static FFI library
cargo build -p liter-llm-ffi --release

# Go build will automatically link against the static library
cd packages/go
go build -v
```

### Building with the Native Library

The Go module wraps the `liter-llm-ffi` C library through cgo, so a C toolchain and the native library are required at build time.

#### Option 1: Download a Pre-built Release

Each release publishes `liter-llm-go-v{{ version }}-<platform>.tar.gz` (plus a `.sha256` sidecar) on [GitHub Releases](https://github.com/xberg-io/liter-llm/releases). The archive contains `lib/` (static library `libliter_llm_ffi.a`, the shared library, and `native-static-libs.txt`) and `include/` (`liter_llm.h`).

| Platform         | `<platform>`      |
| ---------------- | ----------------- |
| Linux x86_64     | `linux-x86_64`    |
| Linux arm64      | `linux-aarch64`   |
| macOS arm64      | `macos-arm64`     |
| macOS x86_64     | `macos-x86_64`    |
| Windows x86_64   | `windows-x86_64`  |
| Windows arm64    | `windows-arm64`   |

Linux builds target glibc (`*-unknown-linux-gnu`). There are no musl builds; on Alpine and other musl distributions, build the library yourself (Option 2). macOS binaries use a deployment target of 11.0.

```bash
# Example: Linux x86_64
curl -LO https://github.com/xberg-io/liter-llm/releases/download/v{{ version }}/liter-llm-go-v{{ version }}-linux-x86_64.tar.gz
tar -xzf liter-llm-go-v{{ version }}-linux-x86_64.tar.gz

mkdir -p ~/liter-llm
cp -R liter-llm-go-v{{ version }}-linux-x86_64/lib liter-llm-go-v{{ version }}-linux-x86_64/include ~/liter-llm/
```

Alternatively, `go run {{ package_name }}/cmd/setup` downloads and checksum-verifies the archive for your platform into a per-user cache and writes a cgo link shim into the current package. Run it with `-print-env` to print `CGO_CFLAGS`/`CGO_LDFLAGS` exports instead, or `-lib-dir .lib` to extract the libraries into a directory without writing the shim.

#### Option 2: Build the Library Yourself

```bash
git clone https://github.com/xberg-io/liter-llm.git
cd liter-llm
cargo build -p liter-llm-ffi --release

mkdir -p ~/liter-llm/lib ~/liter-llm/include
cp target/release/libliter_llm_ffi.* ~/liter-llm/lib/
cp crates/liter-llm-ffi/include/liter_llm.h ~/liter-llm/include/
```

#### Pointing cgo at the library

The module's cgo directives add `-L${SRCDIR}/.lib/<platform>` (and an rpath on Linux and macOS) for the platform directories listed above, and `-I${SRCDIR}/include` for the header. For any other location, set the flags in the environment:

```bash
export CGO_CFLAGS="-I$HOME/liter-llm/include"
export CGO_LDFLAGS="-L$HOME/liter-llm/lib -lliter_llm_ffi"
go build
```

Windows (cmd):

```bat
set CGO_CFLAGS=-I%USERPROFILE%\liter-llm\include
set CGO_LDFLAGS=-L%USERPROFILE%\liter-llm\lib -lliter_llm_ffi
go build
```

#### Static vs. dynamic linking

The linker picks the file from the `-L` directory itself:

- **Static** (`libliter_llm_ffi.a`) produces a self-contained binary. If the directory also holds the shared library, the linker prefers it; to force static linking, keep only the `.a` file in the directory or pass the full path (`CGO_LDFLAGS="$HOME/liter-llm/lib/libliter_llm_ffi.a"`).
- **Dynamic** (`.so` / `.dylib` / `.dll`) keeps the binary small but needs the library on the runtime loader path: `LD_LIBRARY_PATH` on Linux, `DYLD_LIBRARY_PATH` on macOS, `PATH` on Windows. An rpath baked in by the module's cgo directives covers the `.lib/<platform>` directories only.

Static linking also needs the system libraries the Rust runtime depends on. The generated cgo preamble links them automatically on macOS (`-framework Security -framework CoreFoundation -liconv`), Linux (`-lm -ldl -lpthread -lrt`) and Windows (`-lws2_32 -luserenv -lbcrypt -lntdll -ladvapi32 -lkernel32`). If a link still reports undefined symbols, or you link outside the generated preamble, read `lib/native-static-libs.txt` (cargo's `--print native-static-libs` output) and pass what it lists through `CGO_LDFLAGS`, for example:

```bash
CGO_LDFLAGS="-L$HOME/liter-llm/lib -lliter_llm_ffi $(cat $HOME/liter-llm/lib/native-static-libs.txt)" go build
```

#### Slim build

The default FFI build enables `native-http`, `full`, `opendal-cache`, `tokenizer`, and `tower`. For a smaller library without the optional providers, the OpenDAL cache backends, the tokenizer, and the tower middleware stack:

```bash
cargo build -p liter-llm-ffi --release --no-default-features --features native-http
```

Link it as above via `CGO_LDFLAGS`. The module's cgo directives define `LITERLLM_FEATURE_*` macros for the full feature set; a slim library lacks the corresponding symbols, so features that depend on the dropped components (middleware keys in `CreateClientFromJSON`, for example) are unavailable.

### System Requirements

- **Go 1.26+** required, with cgo enabled and a C toolchain
- macOS 11.0 or later on Apple platforms
- API keys via environment variables (e.g. `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`)

## Quickstart

{% raw %}

```go
package main

import (
	"encoding/json"
	"fmt"
	"log"
	"os"

	literllm "github.com/xberg-io/liter-llm/packages/go/v2"
)

func main() {
	client, err := literllm.CreateClient(os.Getenv("OPENAI_API_KEY"), nil, nil, nil, nil)
	if err != nil {
		log.Fatal(err)
	}
	defer client.Free()

	var req literllm.ChatCompletionRequest
	if err := json.Unmarshal([]byte(`{
		"model": "openai/gpt-4o-mini",
		"messages": [{"role": "user", "content": "Hello!"}]
	}`), &req); err != nil {
		log.Fatal(err)
	}

	resp, err := client.Chat(req)
	if err != nil {
		log.Fatalf("chat failed: %v", err)
	}

	if len(resp.Choices) > 0 && resp.Choices[0].Message.Content != nil {
		fmt.Println(*resp.Choices[0].Message.Content)
	}
}
```

{% endraw %}

Build and run:

```bash
CGO_CFLAGS="-I$HOME/liter-llm/include" CGO_LDFLAGS="-L$HOME/liter-llm/lib -lliter_llm_ffi" go build
./myapp
```

## Examples

### Context, Cancellation, and Deadlines

Every network call has an `XxxWithContext(ctx, ...)` variant; the plain `Xxx(...)` uses `context.Background()`. Cancelling the context or hitting its deadline aborts the in-flight native request and returns an error wrapping `ctx.Err()`.

{% raw %}

```go
ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
defer cancel()

resp, err := client.ChatWithContext(ctx, req)
if errors.Is(err, context.DeadlineExceeded) {
	log.Fatal("request timed out")
} else if err != nil {
	log.Fatal(err)
}
```

{% endraw %}

### Streaming Responses

`ChatStreamWithContext` returns a stream. Range over `Chan()` to completion, then read `Err()`. Cancelling the context aborts the native read; the channel then closes and `Err()` returns `ctx.Err()`.

{% raw %}

```go
var req literllm.ChatCompletionRequest
if err := json.Unmarshal([]byte(`{
	"model": "openai/gpt-4o-mini",
	"messages": [{"role": "user", "content": "Tell me a story"}]
}`), &req); err != nil {
	log.Fatal(err)
}

stream, err := client.ChatStreamWithContext(ctx, req)
if err != nil {
	log.Fatal(err)
}

for chunk := range stream.Chan() {
	if len(chunk.Choices) > 0 && chunk.Choices[0].Delta.Content != nil {
		fmt.Print(*chunk.Choices[0].Delta.Content)
	}
}
if err := stream.Err(); err != nil {
	log.Fatal(err)
}
```

{% endraw %}

### Error Handling

Errors are `*literllm.Error` (fields `Code`, `Message`, `StatusCode`, `IsTransient`, `ErrorType`) and wrap sentinel errors such as `ErrRateLimited`, `ErrAuthentication`, `ErrTimeout`, `ErrBudgetExceeded`, and `ErrContextWindowExceeded`.

{% raw %}

```go
resp, err := client.Chat(req)
if err != nil {
	var apiErr *literllm.Error
	switch {
	case errors.Is(err, literllm.ErrRateLimited):
		log.Printf("rate limited, retry later: %v", err)
	case errors.As(err, &apiErr):
		log.Printf("%s (status %d, transient=%v)", apiErr.Message, apiErr.StatusCode, apiErr.IsTransient)
	default:
		log.Fatal(err)
	}
}
```

{% endraw %}

### Client from JSON Configuration

`CreateClientFromJSON` accepts the same keys as the config file documented in the [configuration guide](https://docs.liter-llm.xberg.io):

| Key                                                                             | Description                                                              |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `api_key`, `base_url`, `model_hint`, `timeout_secs`, `max_retries`, `extra_headers` | Client settings                                                      |
| `cache`                                                                         | `max_entries`, `ttl_seconds`, `backend`, `backend_config`                |
| `budget`                                                                        | `global_limit` and `model_limits` in USD; `enforcement` is `"hard"` or `"soft"` |
| `rate_limit`                                                                    | `rpm`, `tpm`, `window_seconds`                                           |
| `in_flight_limit`                                                               | `max_in_flight`                                                          |
| `cooldown_secs`, `health_check_secs`, `cost_tracking`, `tracing`                | Middleware toggles                                                       |
| `providers`                                                                     | Custom providers: `name`, `base_url`, `auth_header`, `model_prefixes`    |

Middleware keys are applied (they require a library built with the `tower` feature, which the default build includes).

{% raw %}

```go
client, err := literllm.CreateClientFromJSON(`{
	"api_key": "sk-...",
	"budget": {
		"global_limit": 50.0,
		"model_limits": {"openai/gpt-4o": 10.0},
		"enforcement": "hard"
	}
}`)
if err != nil {
	log.Fatal(err)
}
defer client.Free()
```

{% endraw %}

When the budget is exhausted under `"hard"` enforcement, calls fail with an error that matches `literllm.ErrBudgetExceeded`.

### Custom `base_url` and Model Prefixes

With a custom `base_url`, the model string is sent to the server verbatim. Set `model_hint` to the provider name to have that one `provider/` prefix stripped: with `"model_hint": "openai"`, `openai/gpt-4o-mini` is sent as `gpt-4o-mini`, while `meta-llama/Llama-3` is left untouched.

{% raw %}

```go
client, err := literllm.CreateClientFromJSON(`{
	"api_key": "none",
	"base_url": "http://localhost:8000/v1",
	"model_hint": "openai"
}`)
```

{% endraw %}

### Multiple Providers

{% raw %}

```go
for _, model := range []string{
	"openai/gpt-4o-mini",
	"anthropic/claude-3-5-sonnet-20241022",
	"groq/llama-3.1-70b-versatile",
} {
	var req literllm.ChatCompletionRequest
	if err := json.Unmarshal([]byte(fmt.Sprintf(`{
		"model": %q,
		"messages": [{"role": "user", "content": "Hello!"}]
	}`, model)), &req); err != nil {
		log.Fatal(err)
	}

	resp, err := client.Chat(req)
	if err != nil {
		log.Printf("%s failed: %v", model, err)
		continue
	}
	if len(resp.Choices) > 0 && resp.Choices[0].Message.Content != nil {
		fmt.Printf("%s: %s\n", model, *resp.Choices[0].Message.Content)
	}
}
```

{% endraw %}

{% include 'partials/proxy_server.md' %}

## API Reference

- **[Documentation](https://docs.liter-llm.xberg.io)** -- Full docs and API reference
- **GoDoc**: [pkg.go.dev/{{ package_name }}](https://pkg.go.dev/{{ package_name }})
- **Provider Registry**: [schemas/providers.json](https://github.com/xberg-io/liter-llm/blob/main/schemas/providers.json)
- **GitHub Repository**: [github.com/xberg-io/liter-llm](https://github.com/xberg-io/liter-llm)

## Part of Xberg.io

- [Xberg](https://github.com/xberg-io/xberg) — document intelligence: text, tables, metadata from 101 formats with optional OCR.
- [Xberg Enterprise](https://github.com/xberg-io/xberg-enterprise) — managed extraction API with SDKs, dashboards, and observability.
- [crawlberg](https://github.com/xberg-io/crawlberg) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/xberg-io/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/xberg-io/liter-llm) — universal LLM API client with native bindings for 14 languages and 165 providers.
- [tree-sitter-language-pack](https://github.com/xberg-io/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/xberg-io/alef) — the polyglot binding generator that produces every per-language binding across the 5 polyglot repos.
- [Discord](https://discord.gg/xt9WY3GnKR) — community, roadmap, announcements.

## Troubleshooting

| Issue                                                                   | Fix                                                                                                                                     |
| ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `ld returned 1 exit status` or `undefined reference to 'liter_llm_...'` | Library not found, or system libraries missing for static linking. Set `CGO_LDFLAGS="-L/path/to/lib -lliter_llm_ffi"` and add the libraries listed in `lib/native-static-libs.txt`. |
| `error while loading shared libraries` / `dyld: Library not loaded`     | Dynamic linking: add the library directory to `LD_LIBRARY_PATH` (Linux), `DYLD_LIBRARY_PATH` (macOS), or `PATH` (Windows).             |
| `cannot find -lliter_llm_ffi`                                           | Download from [GitHub Releases](https://github.com/xberg-io/liter-llm/releases) or build: `cargo build -p liter-llm-ffi --release` |
| `401 Unauthorized`                                                      | API key not set. Export `OPENAI_API_KEY` (or equivalent) before running.                                                                |
| `unknown provider`                                                      | Check the [provider registry](https://github.com/xberg-io/liter-llm/blob/main/schemas/providers.json) for the correct prefix.      |

## Testing / Tooling

- `task go:lint` — runs `gofmt` and `golangci-lint`
- `task go:test` — executes `go test ./...` (after building the static FFI library)
- `task e2e:go:verify` — regenerates fixtures and runs `go test ./...` inside `e2e/go`

Need help? Open an issue at [github.com/xberg-io/liter-llm/issues](https://github.com/xberg-io/liter-llm/issues).
