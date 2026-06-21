<!-- snippet:compile-only -->

```typescript
import { createClient } from "@kreuzberg/liter-llm";
import { readFileSync } from "node:fs";

const client = createClient(process.env.OPENAI_API_KEY!);
// file is a base64-encoded string, not raw bytes.
const file = readFileSync("data.jsonl").toString("base64");
const response = await client.createFile({
  file,
  filename: "data.jsonl",
  purpose: "batch",
});
console.log(`File ID: ${response.id}`);
console.log(`Size: ${response.bytes} bytes`);
```
