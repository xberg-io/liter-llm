<!-- snippet:compile-only -->

```typescript
import init, { createClient, WasmChatCompletionRequest } from "@kreuzberg/liter-llm-wasm";

await init();

const client = createClient(process.env.OPENAI_API_KEY!);

const request = WasmChatCompletionRequest.default();
request.model = "openai/gpt-4o";
request.messages = [{ role: "user", content: "Tell me a story" }];
request.stream = true;

// chatStream resolves to an array of chunks materialised by the WASM binding.
const chunks = await client.chatStream(request);
for (const chunk of chunks) {
  process.stdout.write(chunk.choices?.[0]?.delta?.content ?? "");
}
console.log();
```
