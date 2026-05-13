```typescript
import init, { createClient, WasmChatCompletionRequest } from "@kreuzberg/liter-llm-wasm";

await init();

const client = createClient(process.env.OPENAI_API_KEY!);

const request = WasmChatCompletionRequest.default();
request.model = "openai/gpt-4o";
request.messages = [{ role: "user", content: "Hello" }];

try {
  const response = await client.chat(request);
  console.log(response.choices[0].message.content);
} catch (err) {
  // The WASM binding rejects with a JsValue built from the Rust error's
  // Display impl -- a plain string message. Match on substrings.
  const message = err instanceof Error ? err.message : String(err);
  const lower = message.toLowerCase();
  if (lower.includes("authentication")) {
    console.error("auth failed:", message);
  } else if (lower.includes("rate") || lower.includes("429")) {
    console.error("rate limited:", message);
  } else if (lower.includes("budget")) {
    console.error("budget exceeded:", message);
  } else {
    console.error("llm error:", message);
  }
}
```
