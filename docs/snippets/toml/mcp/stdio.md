```toml title="liter-llm-proxy.toml"
[mcp]
stdio_trust_local = true
```

Use `stdio_trust_local = true` only for trusted local clients. To enforce a virtual-key policy instead, set `stdio_key_id` to an existing `[[keys]].key`.

```json title="claude_desktop_config.json (stdio)"
{
  "mcpServers": {
    "liter-llm": {
      "command": "liter-llm",
      "args": ["mcp", "--config", "/absolute/path/to/liter-llm-proxy.toml"],
      "env": {
        "OPENAI_API_KEY": "sk-...",
        "ANTHROPIC_API_KEY": "sk-ant-..."
      }
    }
  }
}
```
