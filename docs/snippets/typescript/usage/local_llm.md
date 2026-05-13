```typescript
import { createClient } from "@kreuzberg/liter-llm-node";

// No API key needed for local providers
const client = createClient("", "http://localhost:11434/v1");

const response = await client.chat({
  model: "ollama/qwen2:0.5b",
  messages: [{ role: "user", content: "Hello!" }],
});
console.log(response.choices[0].message.content);
```
