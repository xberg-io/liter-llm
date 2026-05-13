<!-- snippet:compile-only -->

```typescript
import init, { createClient, WasmCreateBatchRequest } from "@kreuzberg/liter-llm-wasm";

await init();

const client = createClient(process.env.OPENAI_API_KEY!);

const request = WasmCreateBatchRequest.default();
request.inputFileId = "file-abc123";
request.endpoint = "/v1/chat/completions";
request.completionWindow = "24h";

const response = await client.createBatch(request);
console.log(`Batch ID: ${response.id}`);
console.log(`Status: ${response.status}`);
```
