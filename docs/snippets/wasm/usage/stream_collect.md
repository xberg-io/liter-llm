<!-- snippet:compile-only -->

```typescript
import init, { createClient, WasmChatCompletionRequest } from "@kreuzberg/liter-llm-wasm";

await init();

const client = createClient(process.env.OPENAI_API_KEY!);

const request = WasmChatCompletionRequest.default();
request.model = "openai/gpt-4o";
request.messages = [{ role: "user", content: "Explain quantum computing briefly" }];
request.stream = true;

const stream = await client.chatStream(request);
let fullText = "";
while (true) {
  const chunk = await stream.next();
  if (chunk === null) {
    break;
  }
  const delta = chunk.choices?.[0]?.delta?.content;
  if (delta) {
    fullText += delta;
    process.stdout.write(delta);
  }
}
console.log();
console.log(`Full response length: ${fullText.length} characters`);
```
