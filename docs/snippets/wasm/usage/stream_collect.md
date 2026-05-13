```typescript
import init, { createClient, WasmChatCompletionRequest } from "@kreuzberg/liter-llm-wasm";

await init();

const client = createClient(process.env.OPENAI_API_KEY!);

const request = WasmChatCompletionRequest.default();
request.model = "openai/gpt-4o";
request.messages = [{ role: "user", content: "Explain quantum computing briefly" }];
request.stream = true;

const chunks = await client.chatStream(request);
let fullText = "";
for (const chunk of chunks) {
  const delta = chunk.choices?.[0]?.delta?.content;
  if (delta) {
    fullText += delta;
    process.stdout.write(delta);
  }
}
console.log();
console.log(`Full response length: ${fullText.length} characters`);
```
