<!-- snippet:compile-only -->

```typescript
import init, { createClient, WasmChatCompletionRequest } from "@xberg-io/liter-llm-wasm";

await init();

const client = createClient(process.env.OPENAI_API_KEY!);

const request = WasmChatCompletionRequest.default();
request.model = "openai/gpt-4o";
request.messages = [{ role: "user", content: "Tell me a story" }];
request.stream = true;

const stream = await client.chatStream(request);
while (true) {
  const chunk = await stream.next();
  if (chunk === null) {
    break;
  }
  process.stdout.write(chunk.choices?.[0]?.delta?.content ?? "");
}
console.log();
```
