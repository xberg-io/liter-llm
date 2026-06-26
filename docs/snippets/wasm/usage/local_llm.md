<!-- snippet:compile-only -->

```typescript
import init, {
  createClient,
  WasmChatCompletionRequest,
  WasmMessage,
  WasmUserContent,
} from "@xberg-io/liter-llm-wasm";

await init();

// No API key needed for local providers
const client = createClient("", "http://localhost:11434/v1");

const request = WasmChatCompletionRequest.default();
request.model = "ollama/qwen2:0.5b";

const message = WasmMessage.User(new WasmUserContent.Text("Hello!"));
request.messages = [message];

const response = await client.chat(request);
console.log(response.choices[0].message.content);
```
