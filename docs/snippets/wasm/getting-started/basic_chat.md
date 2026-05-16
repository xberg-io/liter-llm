<!-- snippet:compile-only -->

```typescript
import init, { createClient, WasmChatCompletionRequest } from "@kreuzberg/liter-llm-wasm";

await init();

const client = createClient(process.env.OPENAI_API_KEY!);

const request = WasmChatCompletionRequest.default();
request.model = "openai/gpt-4o";
request.messages = [{ role: "user", content: "Hello!" }];

const response = await client.chat(request);
console.log(response.choices[0].message.content);
```
