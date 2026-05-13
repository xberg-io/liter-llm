```typescript
import { createClient } from "@kreuzberg/liter-llm-node";

// Positional args: apiKey, baseUrl?, timeoutSecs?, maxRetries?, modelHint?
const client = createClient(
  process.env.OPENAI_API_KEY!,
  undefined, // override provider base URL
  60,        // request timeout in seconds
  3,         // retry on transient failures
  "openai",  // pre-resolve provider at construction
);

const response = await client.chat({
  model: "openai/gpt-4o",
  messages: [{ role: "user", content: "Hello!" }],
});
console.log(response.choices[0].message.content);
```
