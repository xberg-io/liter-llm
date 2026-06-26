<!-- snippet:compile-only -->

```typescript
import init, { createClient, WasmRerankRequest } from "@xberg-io/liter-llm-wasm";

await init();

const client = createClient(process.env.COHERE_API_KEY!);

const request = WasmRerankRequest.default();
request.model = "cohere/rerank-v3.5";
request.query = "What is the capital of France?";
request.documents = [
  "Paris is the capital of France.",
  "Berlin is the capital of Germany.",
  "London is the capital of England.",
];

const response = await client.rerank(request);
for (const result of response.results) {
  console.log(`Index: ${result.index}, Score: ${result.relevanceScore.toFixed(4)}`);
}
```
