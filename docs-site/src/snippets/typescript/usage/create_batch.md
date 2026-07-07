<!-- snippet:compile-only -->

```typescript
import { createClient } from "@xberg-io/liter-llm";

const client = createClient(process.env.OPENAI_API_KEY!);
const response = await client.createBatch({
  inputFileId: "file-abc123",
  endpoint: "/v1/chat/completions",
  completionWindow: "24h",
});
console.log(`Batch ID: ${response.id}`);
console.log(`Status: ${response.status}`);
```
