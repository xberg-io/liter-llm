<!-- snippet:compile-only -->

```typescript
import { createClient } from "@kreuzberg/liter-llm-node";

const client = createClient(process.env.OPENAI_API_KEY!);
const chunks = await client.chatStream({
  model: "openai/gpt-4o",
  messages: [{ role: "user", content: "Tell me a story" }],
});

for await (const chunk of chunks) {
  process.stdout.write(chunk.choices?.[0]?.delta?.content ?? "");
}
console.log();
```
