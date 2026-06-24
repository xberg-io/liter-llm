---
title: liter-llm
description: "liter-llm – Universal LLM API client. One Rust core, 14 native language bindings, 143 runtime providers, an OpenAI-compatible proxy, and a built-in MCP server."
---

## Liter-llm

A universal LLM API client with a Rust core and 14 native language bindings (plus a shared C/FFI surface). One surface across 143 runtime providers — chat, streaming, embeddings, rerank, image generation, speech, transcription, OCR, search, files, batches, moderation — plus an OpenAI-compatible proxy server and a Model Context Protocol server, both shipped in the same binary.

<div class="hero-badges" markdown>

[:material-lightning-bolt: Quick Start](getting-started/installation.md){ .md-button .md-button--primary }
[:material-package-variant: Installation](getting-started/installation.md){ .md-button }
[:material-feature-search-outline: Providers](providers.md){ .md-button }
[:fontawesome-brands-discord: Join our Community](https://discord.gg/xt9WY3GnKR){ .md-button }

</div>

---

### Why liter-llm

<div class="grid cards" markdown>

- :material-router-network:{ .lg .middle } **143 Providers**

  OpenAI, Anthropic, Google, Bedrock, Vertex, Azure, Mistral, Cohere, GitHub Copilot, and many more — one client, one model-prefix routing scheme.

- :material-translate:{ .lg .middle } **14 Native Bindings**

  Rust, Python, TypeScript, Go, Java, Kotlin Android, C#, Ruby, PHP, Elixir, Dart, Swift, Zig, and WebAssembly — plus a shared C/FFI surface.

- :material-chat-processing:{ .lg .middle } **Full Endpoint Coverage**

  `LlmClient` covers chat, chat streaming, embeddings, model listing, images, speech, transcription, moderation, rerank, search, and OCR. Files, batches, and responses use dedicated clients.

- :material-server:{ .lg .middle } **Proxy & MCP Server**

  Drop-in OpenAI-compatible proxy with virtual keys, budgets, fallbacks, and observability. Same binary exposes a Model Context Protocol server for AI agents.

- :material-shield-key:{ .lg .middle } **Cloud-Native Auth**

  Azure AD, AWS Bedrock SigV4 with STS/IRSA, Vertex AI service-account OAuth2, GitHub Copilot — automatic token caching, refresh, and rotation.

- :material-shuffle-variant:{ .lg .middle } **Routing & Fallback**

  Round-robin, weighted, latency-based, cost-based, and ordered-fallback strategies. Per-request override or proxy-level config.

</div>

---

### Language Support

| Language              | Package                                                 | Docs                                         |
| :-------------------- | :------------------------------------------------------ | :------------------------------------------- |
| **Rust**              | `cargo add liter-llm`                                   | [API Reference](reference/api-rust.md)       |
| **Python**            | `pip install liter-llm`                                 | [API Reference](reference/api-python.md)     |
| **TypeScript / Node** | `npm install @kreuzberg/liter-llm`                 | [API Reference](reference/api-typescript.md) |
| **WebAssembly**       | `npm install @kreuzberg/liter-llm-wasm`                 | [API Reference](reference/api-wasm.md)       |
| **Go**                | `go get github.com/xberg-io/liter-llm/packages/go` | [API Reference](reference/api-go.md)         |
| **Java**              | Maven Central `dev.kreuzberg.literllm:liter-llm`        | [API Reference](reference/api-java.md)       |
| **Kotlin Android**    | Maven `dev.kreuzberg:liter-llm-android`                 | [API Reference](reference/api-kotlin-android.md) |
| **C#**                | `dotnet add package LiterLlm`                           | [API Reference](reference/api-csharp.md)     |
| **Ruby**              | `gem install liter_llm`                                 | [API Reference](reference/api-ruby.md)       |
| **PHP**               | `composer require xberg-io/liter-llm`              | [API Reference](reference/api-php.md)        |
| **Elixir**            | `{:liter_llm, "~> 1.7"}`                                | [API Reference](reference/api-elixir.md)     |
| **Dart / Flutter**    | `dart pub add liter_llm`                                | [API Reference](reference/api-dart.md)       |
| **Swift**             | Swift Package Manager                                   | [API Reference](reference/api-swift.md)      |
| **Zig**               | `zig fetch --save` from GitHub                          | [API Reference](reference/api-zig.md)        |
| **C (FFI)**           | Shared library + header                                 | [API Reference](reference/api-c.md)          |
| **CLI**               | `cargo install liter-llm-cli`                           | [Proxy Server](server/proxy-server.md)       |
| **Docker**            | `ghcr.io/xberg-io/liter-llm`                       | [Proxy Server](server/proxy-server.md)       |

---

### Feature Support

All bindings are generated from one Rust core, so endpoint coverage is uniform; the only real difference is the call idiom (async/await vs. sync vs. an explicit `_async` suffix). The `C/FFI` column is the shared native surface the C-ABI bindings (Go, Java, Kotlin, C#, Dart, Swift, Zig) are built on.

| Feature | Rust | Python | TS / Node | Go | Java | Kotlin | C# | Ruby | PHP | Elixir | Dart | Swift | Zig | WASM | C/FFI |
|---------|:----:|:------:|:---------:|:--:|:----:|:------:|:--:|:----:|:---:|:------:|:----:|:-----:|:---:|:----:|:-----:|
| **Chat** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **Streaming** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **Tool calling** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **Embeddings** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **Vision input** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **Structured output** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **Audio output** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **Image output** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **Call idiom** | `.await` | `await` | `await` | sync | sync | sync | `await` | `_async` | sync | `_async` | `await` | `async` | sync | `await` | sync |

---

### Quick Example

=== "Rust"

    ```rust title="src/main.rs"
    use liter_llm::{ChatCompletionRequest, ClientConfigBuilder, DefaultClient, LlmClient, Message};

    #[tokio::main]
    async fn main() -> Result<(), Box<dyn std::error::Error>> {
        let config = ClientConfigBuilder::new(std::env::var("OPENAI_API_KEY")?).build();
        let client = DefaultClient::new(config, None)?;

        let request = ChatCompletionRequest::builder("openai/gpt-4o-mini")
            .add_user_message("Summarize liter-llm in one sentence.")
            .build()?;

        let response = client.chat(request).await?;
        println!("{}", response.choices[0].message.content_text().unwrap_or(""));
        Ok(())
    }
    ```

=== "Python"

    ```python title="main.py"
    import asyncio
    import os
    from liter_llm import create_client

    async def main():
        client = create_client(api_key=os.environ["OPENAI_API_KEY"])
        response = await client.chat({
            "model": "openai/gpt-4o-mini",
            "messages": [{"role": "user", "content": "Summarize liter-llm in one sentence."}],
        })
        print(response["choices"][0]["message"]["content"])

    asyncio.run(main())
    ```

=== "TypeScript"

    ```typescript title="index.ts"
    import { createClient } from "@kreuzberg/liter-llm";

    const client = createClient(process.env.OPENAI_API_KEY!);

    const response = await client.chat({
      model: "openai/gpt-4o-mini",
      messages: [{ role: "user", content: "Summarize liter-llm in one sentence." }],
    });

    console.log(response.choices[0].message.content);
    ```

---

### Part of Kreuzberg.dev

<div class="grid cards" markdown>

- :material-file-document-multiple:{ .lg .middle } **[Kreuzberg](https://docs.kreuzberg.dev)**

  Document intelligence — text, tables, and metadata from 96 file formats with optional OCR.

- :material-cloud:{ .lg .middle } **[Kreuzberg Cloud](https://docs.kreuzberg.cloud)**

  Managed document-extraction API with SDKs, dashboards, and observability built in.

- :material-spider-web:{ .lg .middle } **[Kreuzcrawl](https://docs.kreuzcrawl.kreuzberg.dev)**

  High-performance web crawling and scraping with always-on HTML→Markdown and headless-Chrome fallback.

- :material-language-html5:{ .lg .middle } **[html-to-markdown](https://docs.html-to-markdown.kreuzberg.dev)**

  Fast, lossless HTML→Markdown engine — Rust core, the same conversion used by Kreuzcrawl.

- :material-code-tags:{ .lg .middle } **[tree-sitter-language-pack](https://docs.tree-sitter-language-pack.kreuzberg.dev)**

  306 tree-sitter grammars and code-intelligence primitives.

- :fontawesome-brands-discord:{ .lg .middle } **[Discord](https://discord.gg/xt9WY3GnKR)**

  Join the Kreuzberg community for help, roadmap discussion, and announcements.

</div>

---

### Explore the Docs

<div class="grid cards" markdown>

- :material-rocket-launch:{ .lg .middle } **Get Started**

  Install liter-llm for your language, set an API key, and make your first call.

  [:octicons-arrow-right-24: Installation](getting-started/installation.md)

- :material-book-open-variant:{ .lg .middle } **Guides**

  Chat, embeddings, media, search, fallback routing, authentication, and the proxy/MCP servers.

  [:octicons-arrow-right-24: Chat & Streaming](usage/chat.md)

- :material-puzzle-outline:{ .lg .middle } **Concepts**

  Architecture, feature flags, tokenizer model, and cost-estimation pipeline.

  [:octicons-arrow-right-24: Architecture](concepts/architecture.md)

- :material-api:{ .lg .middle } **Reference**

  Per-language API docs, the configuration schema, type catalogue, and error matrix.

  [:octicons-arrow-right-24: References](reference/api-rust.md)

- :material-router-network:{ .lg .middle } **Providers**

  Browse all 143 runtime providers, model prefixes, auth modes, and endpoint coverage.

  [:octicons-arrow-right-24: Provider Registry](providers.md)

- :material-server-network:{ .lg .middle } **Proxy & MCP**

  Run the OpenAI-compatible proxy and the Model Context Protocol server from one binary.

  [:octicons-arrow-right-24: Proxy Server](server/proxy-server.md)

</div>

---

### Getting Help

- **Bugs & feature requests** — [Open an issue on GitHub](https://github.com/xberg-io/liter-llm/issues)
- **Community chat** — [Join the Discord](https://discord.gg/xt9WY3GnKR)
- **Contributing** — [Read the contributor guide](contributing.md)
