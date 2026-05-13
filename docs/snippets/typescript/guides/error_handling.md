```typescript
import { createClient } from "@kreuzberg/liter-llm-node";

const client = createClient(process.env.OPENAI_API_KEY!);

try {
  const response = await client.chat({
    model: "openai/gpt-4o",
    messages: [{ role: "user", content: "Hello" }],
  });
  console.log(response.choices[0].message.content);
} catch (err) {
  // Errors surface as plain JS Error objects -- the message is the Rust
  // error's Display form (e.g. "Authentication failed: invalid api key").
  // Match by substring or rely on the upstream HTTP status text.
  if (err instanceof Error) {
    const msg = err.message.toLowerCase();
    if (msg.includes("authentication")) {
      // 401/403 -- rotate the key.
      console.error("auth failed:", err.message);
    } else if (msg.includes("rate") || msg.includes("429")) {
      // 429 -- transient, retry or fall back.
      console.error("rate limited:", err.message);
    } else if (msg.includes("budget")) {
      console.error("budget exceeded:", err.message);
    } else {
      console.error("llm error:", err.message);
    }
  }
}
```
