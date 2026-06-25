# liter-llm — Go

{% include 'partials/badges.html' %}
{% include 'partials/banner.html' %}
{% include 'partials/discord.html' %}

Universal LLM API client for Go. Access 143 LLM providers through a single interface backed by the Rust core.

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

### Building with Static Libraries

When building outside the liter-llm monorepo, provide the static library (`.a` on Unix, `.lib` on Windows).

#### Option 1: Download Pre-built Static Library

Download from [GitHub Releases](https://github.com/xberg-io/liter-llm/releases):

```bash
# Example: Linux x86_64
curl -LO https://github.com/xberg-io/liter-llm/releases/download/v{{ version }}/go-ffi-linux-x86_64.tar.gz
tar -xzf go-ffi-linux-x86_64.tar.gz

mkdir -p ~/liter-llm/lib
cp liter-llm-ffi/lib/libliter_llm_ffi.a ~/liter-llm/lib/
```

Then build with `CGO_LDFLAGS`:

```bash
# Linux/macOS
CGO_LDFLAGS="-L$HOME/liter-llm/lib -lliter_llm_ffi" go build

# Windows (MSVC)
set CGO_LDFLAGS=-L%USERPROFILE%\liter-llm\lib -lliter_llm_ffi
go build
```

#### Option 2: Build Static Library Yourself

```bash
git clone https://github.com/xberg-io/liter-llm.git
cd liter-llm

cargo build -p liter-llm-ffi --release

mkdir -p ~/liter-llm/lib
cp target/release/libliter_llm_ffi.a ~/liter-llm/lib/

cd ~/my-go-project
CGO_LDFLAGS="-L$HOME/liter-llm/lib -lliter_llm_ffi" go build
```

### System Requirements

- **Go 1.21+** required
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

	literllm "github.com/xberg-io/liter-llm/packages/go"
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
CGO_LDFLAGS="-L$HOME/liter-llm/lib -lliter_llm_ffi" go build
./myapp
```

## Examples

### Streaming Responses

{% raw %}

```go
var req literllm.ChatCompletionRequest
if err := json.Unmarshal([]byte(`{
	"model": "openai/gpt-4o-mini",
	"messages": [{"role": "user", "content": "Tell me a story"}]
}`), &req); err != nil {
	log.Fatal(err)
}

stream, err := client.ChatStream(req)
if err != nil {
	log.Fatal(err)
}

for chunk := range stream {
	if len(chunk.Choices) > 0 && chunk.Choices[0].Delta.Content != nil {
		fmt.Print(*chunk.Choices[0].Delta.Content)
	}
}
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

## Part of Kreuzberg.dev

- [Kreuzberg](https://github.com/xberg-io/kreuzberg) — document intelligence: text, tables, metadata from 91+ formats with optional OCR.
- [Xberg Enterprise](https://github.com/xberg-io/xberg-enterprise) — managed extraction API with SDKs, dashboards, and observability.
- [kreuzcrawl](https://github.com/xberg-io/kreuzcrawl) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/xberg-io/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/xberg-io/liter-llm) — universal LLM API client with native bindings for 14 languages and 143 providers.
- [tree-sitter-language-pack](https://github.com/xberg-io/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/xberg-io/alef) — the polyglot binding generator that produces every per-language binding across the 5 polyglot repos.
- [Discord](https://discord.gg/xt9WY3GnKR) — community, roadmap, announcements.

## Troubleshooting

| Issue                                                                   | Fix                                                                                                                                     |
| ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `ld returned 1 exit status` or `undefined reference to 'liter_llm_...'` | Static library not found. Set `CGO_LDFLAGS="-L/path/to/lib -lliter_llm_ffi" go build`                                                   |
| `cannot find -lliter_llm_ffi`                                           | Download from [GitHub Releases](https://github.com/xberg-io/liter-llm/releases) or build: `cargo build -p liter-llm-ffi --release` |
| `401 Unauthorized`                                                      | API key not set. Export `OPENAI_API_KEY` (or equivalent) before running.                                                                |
| `unknown provider`                                                      | Check the [provider registry](https://github.com/xberg-io/liter-llm/blob/main/schemas/providers.json) for the correct prefix.      |

## Testing / Tooling

- `task go:lint` — runs `gofmt` and `golangci-lint`
- `task go:test` — executes `go test ./...` (after building the static FFI library)
- `task e2e:go:verify` — regenerates fixtures and runs `go test ./...` inside `e2e/go`

Need help? Open an issue at [github.com/xberg-io/liter-llm/issues](https://github.com/xberg-io/liter-llm/issues).
