<!-- snippet:compile-only -->

```typescript
import { createClient } from "@xberg-io/liter-llm";

const client = createClient(process.env.OPENAI_API_KEY!);
const chunks = await client.chatStream({
  model: "openai/gpt-4o",
  messages: [{ role: "user", content: "Explain quantum computing briefly" }],
});

let fullText = "";
for await (const chunk of chunks) {
  const delta = chunk.choices?.[0]?.delta?.content;
  if (delta) {
    fullText += delta;
    process.stdout.write(delta);
  }
}
console.log();
console.log(`Full response length: ${fullText.length} characters`);
```
