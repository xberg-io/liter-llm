<!-- snippet:compile-only -->

```typescript
import { createClient } from "@kreuzberg/liter-llm-node";

const client = createClient(process.env.OPENAI_API_KEY!);
const response = await client.createResponse({
  model: "openai/gpt-4o",
  input: "Explain quantum computing in one sentence.",
});
console.log(`Status: ${response.status}`);
for (const item of response.output ?? []) {
  console.log(item.content);
}
```
