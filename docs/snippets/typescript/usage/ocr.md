<!-- snippet:compile-only -->

```typescript
import { createClient } from "@kreuzberg/liter-llm";

const client = createClient(process.env.MISTRAL_API_KEY!);
const response = await client.ocr({
  model: "mistral/mistral-ocr-latest",
  document: { type: "document_url", url: "https://example.com/invoice.pdf" },
});

for (const page of response.pages) {
  console.log(`Page ${page.index}: ${page.markdown.slice(0, 100)}...`);
}
```
