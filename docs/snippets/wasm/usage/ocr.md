<!-- snippet:compile-only -->

```typescript
import init, { createClient, WasmOcrRequest } from "@kreuzberg/liter-llm-wasm";

await init();

const client = createClient(process.env.MISTRAL_API_KEY!);

const request = WasmOcrRequest.default();
request.model = "mistral/mistral-ocr-latest";
request.document = { type: "document_url", url: "https://example.com/invoice.pdf" };

const response = await client.ocr(request);
for (const page of response.pages) {
  console.log(`Page ${page.index}: ${page.markdown.slice(0, 100)}...`);
}
```
