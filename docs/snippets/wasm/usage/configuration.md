```typescript
import init, { createClient, WasmChatCompletionRequest } from "@kreuzberg/liter-llm-wasm";

await init();

// Positional args: apiKey, baseUrl?, timeoutSecs?, maxRetries?, modelHint?
const client = createClient(
  process.env.OPENAI_API_KEY!,
  undefined, // override provider base URL
  60n,       // request timeout in seconds (u64 -> bigint)
  3,         // retry on transient failures
  "openai",  // pre-resolve provider at construction
);

const request = WasmChatCompletionRequest.default();
request.model = "openai/gpt-4o";
request.messages = [{ role: "user", content: "Hello!" }];

const response = await client.chat(request);
console.log(response.choices[0].message.content);
```
