## Proxy Server

liter-llm also ships as an OpenAI-compatible proxy server with Docker support:

```bash
docker run -p 4000:4000 -e LITER_LLM_MASTER_KEY=sk-your-key ghcr.io/kreuzberg-dev/liter-llm
```

See the [proxy server documentation](https://docs.liter-llm.kreuzberg.dev/server/proxy-server/) for configuration, CLI usage, and MCP integration.
