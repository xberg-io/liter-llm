<!-- snippet:compile-only -->

```typescript
import init, { createClient, WasmCreateImageRequest } from "@kreuzberg/liter-llm-wasm";

await init();

const client = createClient(process.env.OPENAI_API_KEY!);

const request = WasmCreateImageRequest.default();
request.model = "openai/dall-e-3";
request.prompt = "A sunset over mountains";
request.n = 1;
request.size = "1024x1024";

const response = await client.imageGenerate(request);
console.log(response.data?.[0]?.url);
```
