```typescript
import { createClient } from "@kreuzberg/liter-llm-node";

const client = createClient(process.env.OPENAI_API_KEY!);
const response = await client.embed({
  model: "openai/text-embedding-3-small",
  input: ["The quick brown fox jumps over the lazy dog"],
});
console.log(`Dimensions: ${response.data[0].embedding.length}`);
console.log(`First 5 values: ${response.data[0].embedding.slice(0, 5)}`);
```
