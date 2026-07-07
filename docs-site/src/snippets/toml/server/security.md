```toml title="liter-llm-proxy.toml"
[security]
outbound_policy = "deny_private"

# Use allowlist mode to restrict outbound requests to specific provider origins.
# outbound_policy = "allowlist"
# outbound_allowlist = ["https://api.openai.com", "https://api.anthropic.com"]
```
