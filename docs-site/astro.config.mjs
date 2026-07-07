// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import starlightLlmsTxt from "starlight-llms-txt";
// Local link during migration; switch to "^0.1.0" once @xberg-io/docs-theme is published.
import { xbergStarlightConfig } from "@xberg-io/docs-theme";

const API_LANGUAGES = [
  { label: "Rust", slug: "reference/api-rust" },
  { label: "Python", slug: "reference/api-python" },
  { label: "TypeScript / Node.js", slug: "reference/api-typescript" },
  { label: "Go", slug: "reference/api-go" },
  { label: "Java", slug: "reference/api-java" },
  { label: "Kotlin (Android)", slug: "reference/api-kotlin-android" },
  { label: "C#", slug: "reference/api-csharp" },
  { label: "Ruby", slug: "reference/api-ruby" },
  { label: "PHP", slug: "reference/api-php" },
  { label: "Elixir", slug: "reference/api-elixir" },
  { label: "Dart", slug: "reference/api-dart" },
  { label: "Swift", slug: "reference/api-swift" },
  { label: "Zig", slug: "reference/api-zig" },
  { label: "WebAssembly", slug: "reference/api-wasm" },
  { label: "C / FFI", slug: "reference/api-c" },
];

// https://astro.build/config
export default defineConfig({
  site: "https://docs.liter-llm.xberg.io",
  integrations: [
    starlight(
      xbergStarlightConfig({
        title: "liter-llm",
        description:
          "Universal LLM API client with a Rust core and native bindings for 14 languages and " +
          "143 providers, an OpenAI-compatible proxy server, and a built-in MCP server.",
        githubUrl: "https://github.com/xberg-io/liter-llm",
        editBaseUrl: "https://github.com/xberg-io/liter-llm/edit/main/docs-site/",
        plugins: [starlightLlmsTxt()],
        sidebar: [
          { label: "Home", link: "/" },
          {
            label: "Get Started",
            items: [{ label: "Installation", slug: "getting-started/installation" }],
          },
          {
            label: "Guides",
            items: [
              {
                label: "Core",
                items: [
                  { label: "AI Coding Assistants", slug: "usage/agent-skills" },
                  { label: "Chat & Streaming", slug: "usage/chat" },
                  { label: "Multimodal I/O", slug: "usage/multimodal" },
                  { label: "Embeddings & Rerank", slug: "usage/embeddings" },
                  { label: "Media (Images, Speech, Transcription)", slug: "usage/media" },
                  { label: "Search & OCR", slug: "usage/search-ocr" },
                  { label: "Files & Batches", slug: "usage/files" },
                  { label: "Configuration", slug: "usage/configuration" },
                ],
              },
              {
                label: "Advanced",
                items: [
                  { label: "Authentication", slug: "usage/authentication" },
                  { label: "Batches", slug: "usage/batches" },
                  { label: "Fallback & Routing", slug: "usage/fallback-routing" },
                  { label: "Local LLMs", slug: "usage/local-llms" },
                  { label: "Error Handling", slug: "usage/error-handling" },
                  { label: "Multi-Tenancy", slug: "usage/multi-tenancy" },
                  { label: "Observability", slug: "usage/observability" },
                ],
              },
              {
                label: "Deployment",
                items: [
                  { label: "Proxy Server", slug: "server/proxy-server" },
                  { label: "Proxy Configuration", slug: "server/proxy-configuration" },
                  { label: "Embedding the Proxy", slug: "server/embedding" },
                  { label: "Key Resolvers", slug: "server/key-resolvers" },
                  { label: "MCP Server", slug: "server/mcp-server" },
                  { label: "MCP & IDE Integration", slug: "usage/mcp-integration" },
                ],
              },
            ],
          },
          {
            label: "Concepts",
            items: [
              { label: "Architecture", slug: "concepts/architecture" },
              { label: "Feature Flags", slug: "concepts/feature-flags" },
              { label: "Tokenizer", slug: "concepts/tokenizer" },
              { label: "Cost Estimation", slug: "concepts/cost-estimation" },
            ],
          },
          {
            label: "Reference",
            items: [
              { label: "API", items: API_LANGUAGES },
              { label: "Providers", slug: "providers" },
              { label: "Configuration", slug: "reference/configuration" },
              { label: "Types", slug: "reference/types" },
              { label: "Errors", slug: "reference/errors" },
            ],
          },
          {
            label: "More",
            items: [
              { label: "Contributing", slug: "contributing" },
              { label: "Changelog", slug: "changelog" },
              { label: "Ecosystem", slug: "ecosystem" },
            ],
          },
        ],
      }),
    ),
  ],
});
