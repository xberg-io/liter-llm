<!-- snippet:compile-only -->

```typescript
import init, { createClient, WasmEmbeddingRequest } from "@kreuzberg/liter-llm-wasm";

await init();

const client = createClient(process.env.OPENAI_API_KEY!);

const request = WasmEmbeddingRequest.default();
request.model = "openai/text-embedding-3-small";
request.input = ["The quick brown fox jumps over the lazy dog"];

const response = await client.embed(request);
console.log(`Dimensions: ${response.data[0].embedding.length}`);
console.log(`First 5 values: ${response.data[0].embedding.slice(0, 5)}`);
```
