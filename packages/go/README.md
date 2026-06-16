# liter-llm — Go

<div align="center" style="display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin: 20px 0">
	<!-- Built with -->
	<a href="https://github.com/kreuzberg-dev/alef">
		<img src="https://img.shields.io/badge/Bindings-alef%20%D7%90-007ec6" alt="Bindings" />
	</a>
	<!-- Language Bindings -->
	<a href="https://crates.io/crates/liter-llm">
		<img src="https://img.shields.io/crates/v/liter-llm?label=Rust&color=007ec6" alt="Rust" />
	</a>
	<a href="https://pypi.org/project/liter-llm/">
		<img src="https://img.shields.io/pypi/v/liter-llm?label=Python&color=007ec6" alt="Python" />
	</a>
	<a href="https://www.npmjs.com/package/@kreuzberg/liter-llm-node">
		<img src="https://img.shields.io/npm/v/@kreuzberg/liter-llm-node?label=Node.js&color=007ec6" alt="Node.js" />
	</a>
	<a href="https://www.npmjs.com/package/@kreuzberg/liter-llm-wasm">
		<img src="https://img.shields.io/npm/v/@kreuzberg/liter-llm-wasm?label=WASM&color=007ec6" alt="WASM" />
	</a>
	<a href="https://central.sonatype.com/artifact/dev.kreuzberg.literllm/liter-llm">
		<img src="https://img.shields.io/maven-central/v/dev.kreuzberg.literllm/liter-llm?label=Java&color=007ec6" alt="Java" />
	</a>
	<a href="https://github.com/kreuzberg-dev/liter-llm/tree/main/packages/go">
		<img src="https://img.shields.io/github/v/tag/kreuzberg-dev/liter-llm?label=Go&color=007ec6" alt="Go" />
	</a>
	<a href="https://www.nuget.org/packages/LiterLlm">
		<img src="https://img.shields.io/nuget/v/LiterLlm?label=C%23&color=007ec6" alt="C#" />
	</a>
	<a href="https://packagist.org/packages/kreuzberg-dev/liter-llm">
		<img src="https://img.shields.io/packagist/v/kreuzberg-dev/liter-llm?label=PHP&color=007ec6" alt="PHP" />
	</a>
	<a href="https://rubygems.org/gems/liter_llm">
		<img src="https://img.shields.io/gem/v/liter_llm?label=Ruby&color=007ec6" alt="Ruby" />
	</a>
	<a href="https://hex.pm/packages/liter_llm">
		<img src="https://img.shields.io/hexpm/v/liter_llm?label=Elixir&color=007ec6" alt="Elixir" />
	</a>
	<a href="https://github.com/kreuzberg-dev/liter-llm/pkgs/container/liter-llm">
		<img src="https://img.shields.io/badge/Docker-007ec6?logo=docker&logoColor=white" alt="Docker" />
	</a>
	<a href="https://github.com/kreuzberg-dev/homebrew-tap/blob/main/Formula/liter-llm.rb">
		<img src="https://img.shields.io/badge/Homebrew-007ec6?logo=homebrew&logoColor=white" alt="Homebrew" />
	</a>
	<a href="https://github.com/kreuzberg-dev/liter-llm/tree/main/crates/liter-llm-ffi">
		<img src="https://img.shields.io/badge/C-FFI-007ec6" alt="C FFI" />
	</a>

	<!-- Project Info -->
	<a href="https://github.com/kreuzberg-dev/liter-llm/blob/main/LICENSE">
		<img src="https://img.shields.io/badge/License-MIT-007ec6" alt="License" />
	</a>
	<a href="https://docs.liter-llm.kreuzberg.dev">
		<img src="https://img.shields.io/badge/Docs-liter--llm-007ec6" alt="Docs" />
	</a>
</div>
<div align="center" style="margin: 24px 0 0">
	<a href="https://kreuzberg.dev">
		<img
			alt="kreuzberg.dev"
			src="https://github.com/user-attachments/assets/1b6c6ad7-3b6d-4171-b1c9-f2026cc9deb8"
		/>
	</a>
</div>
<div align="center" style="display: flex; flex-wrap: wrap; gap: 12px; justify-content: center; margin: 28px 0 24px">
	<a href="https://discord.gg/xt9WY3GnKR">
		<img
			height="22"
			src="https://img.shields.io/badge/Discord-Chat-007ec6?logo=discord&logoColor=white"
			alt="Join Discord"
		/>
	</a>
</div>

Universal LLM API client for Go. Access 143 LLM providers through a single interface backed by the Rust core.

> **Version 1.6.1**
> Report issues at [github.com/kreuzberg-dev/liter-llm](https://github.com/kreuzberg-dev/liter-llm/issues).

## What This Package Provides

- **Go module over the Rust client** — context-aware chat, streaming, embeddings, tool calls, search, and OCR.
- **Provider/model routing** — call `provider/model` names without provider-specific client branches.
- **Static-link workflow** — build against `liter-llm-ffi` and ship a self-contained Go binary.
- **Cross-binding parity** — behavior matches the Rust, Python, Node.js, Java, .NET, Ruby, PHP, Elixir, Swift, Dart, Zig, WASM, and C FFI packages.

## Install

### Using Go Modules

```bash
go get github.com/kreuzberg-dev/liter-llm/packages/go@latest
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

Download from [GitHub Releases](https://github.com/kreuzberg-dev/liter-llm/releases):

```bash
# Example: Linux x86_64
curl -LO https://github.com/kreuzberg-dev/liter-llm/releases/download/v1.6.1/go-ffi-linux-x86_64.tar.gz
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
git clone https://github.com/kreuzberg-dev/liter-llm.git
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

```go
package main

import (
	"encoding/json"
	"fmt"
	"log"
	"os"

	literllm "github.com/kreuzberg-dev/liter-llm/packages/go"
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

Build and run:

```bash
CGO_LDFLAGS="-L$HOME/liter-llm/lib -lliter_llm_ffi" go build
./myapp
```

## Examples

### Streaming Responses

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

### Multiple Providers

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

## Proxy Server

liter-llm also ships as an OpenAI-compatible proxy server with Docker support:

```bash
docker run -p 4000:4000 -e LITER_LLM_MASTER_KEY=sk-your-key ghcr.io/kreuzberg-dev/liter-llm
```

See the [proxy server documentation](https://docs.liter-llm.kreuzberg.dev/server/proxy-server/) for configuration, CLI usage, and MCP integration.

## API Reference

- **[Documentation](https://docs.liter-llm.kreuzberg.dev)** -- Full docs and API reference
- **GoDoc**: [pkg.go.dev/github.com/kreuzberg-dev/liter-llm/packages/go](https://pkg.go.dev/github.com/kreuzberg-dev/liter-llm/packages/go)
- **Provider Registry**: [schemas/providers.json](https://github.com/kreuzberg-dev/liter-llm/blob/main/schemas/providers.json)
- **GitHub Repository**: [github.com/kreuzberg-dev/liter-llm](https://github.com/kreuzberg-dev/liter-llm)

## Part of Kreuzberg.dev

- [Kreuzberg](https://github.com/kreuzberg-dev/kreuzberg) — document intelligence: text, tables, metadata from 91+ formats with optional OCR.
- [Kreuzberg Cloud](https://github.com/kreuzberg-dev/kreuzberg-cloud) — managed extraction API with SDKs, dashboards, and observability.
- [kreuzcrawl](https://github.com/kreuzberg-dev/kreuzcrawl) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/kreuzberg-dev/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/kreuzberg-dev/liter-llm) — universal LLM API client with native bindings for 14 languages and 143 providers.
- [tree-sitter-language-pack](https://github.com/kreuzberg-dev/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/kreuzberg-dev/alef) — the polyglot binding generator that produces every per-language binding across the 5 polyglot repos.
- [Discord](https://discord.gg/xt9WY3GnKR) — community, roadmap, announcements.

## Troubleshooting

| Issue                                                                   | Fix                                                                                                                                     |
| ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `ld returned 1 exit status` or `undefined reference to 'liter_llm_...'` | Static library not found. Set `CGO_LDFLAGS="-L/path/to/lib -lliter_llm_ffi" go build`                                                   |
| `cannot find -lliter_llm_ffi`                                           | Download from [GitHub Releases](https://github.com/kreuzberg-dev/liter-llm/releases) or build: `cargo build -p liter-llm-ffi --release` |
| `401 Unauthorized`                                                      | API key not set. Export `OPENAI_API_KEY` (or equivalent) before running.                                                                |
| `unknown provider`                                                      | Check the [provider registry](https://github.com/kreuzberg-dev/liter-llm/blob/main/schemas/providers.json) for the correct prefix.      |

## Testing / Tooling

- `task go:lint` — runs `gofmt` and `golangci-lint`
- `task go:test` — executes `go test ./...` (after building the static FFI library)
- `task e2e:go:verify` — regenerates fixtures and runs `go test ./...` inside `e2e/go`

Need help? Open an issue at [github.com/kreuzberg-dev/liter-llm/issues](https://github.com/kreuzberg-dev/liter-llm/issues).
