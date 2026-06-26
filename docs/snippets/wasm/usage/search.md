<!-- snippet:compile-only -->

```typescript
import init, { createClient, WasmSearchRequest } from "@xberg-io/liter-llm-wasm";

await init();

const client = createClient(process.env.BRAVE_API_KEY!);

const request = WasmSearchRequest.default();
request.model = "brave/web-search";
request.query = "What is Rust programming language?";
request.maxResults = 5;

const response = await client.search(request);
for (const result of response.results) {
  console.log(`${result.title}: ${result.url}`);
}
```
