## Proxy, MCP Server & Plugin

<details>
<summary><strong>Run the OpenAI-compatible proxy or the MCP server</strong></summary>

Beyond the SDK, the `liter-llm` CLI ships an OpenAI-compatible proxy and a Model Context Protocol (MCP) server:

```bash
brew install kreuzberg-dev/tap/liter-llm   # or: cargo install liter-llm-cli
liter-llm api --config liter-llm-proxy.toml   # OpenAI-compatible proxy
liter-llm mcp --transport stdio               # MCP tool server

# or run the proxy without installing:
docker run -p 4000:4000 -e LITER_LLM_MASTER_KEY=sk-your-key ghcr.io/kreuzberg-dev/liter-llm
```

To use the MCP server inside a coding agent, install the **liter-llm plugin** from the [`kreuzberg-dev/plugins`](https://github.com/kreuzberg-dev/plugins) marketplace — it auto-registers the server. See the [MCP server](https://docs.liter-llm.kreuzberg.dev/server/mcp-server/) and [proxy server](https://docs.liter-llm.kreuzberg.dev/server/proxy-server/) guides for configuration, CLI usage, and agent integration.

</details>
