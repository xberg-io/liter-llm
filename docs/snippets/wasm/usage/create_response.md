<!-- snippet:compile-only -->

```typescript
import init, { createClient, WasmCreateResponseRequest } from "@xberg-io/liter-llm-wasm";

await init();

const client = createClient(process.env.OPENAI_API_KEY!);

const request = WasmCreateResponseRequest.default();
request.model = "openai/gpt-4o";
request.input = "Explain quantum computing in one sentence.";

const response = await client.createResponse(request);
console.log(`Status: ${response.status}`);
for (const item of response.output ?? []) {
  console.log(item.content);
}
```
