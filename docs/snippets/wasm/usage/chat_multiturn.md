<!-- snippet:compile-only -->

```typescript
import init, { createClient, WasmChatCompletionRequest } from "@xberg-io/liter-llm-wasm";

await init();

const client = createClient(process.env.OPENAI_API_KEY!);
const messages: Array<{ role: string; content: string }> = [
  { role: "system", content: "You are a helpful assistant." },
  { role: "user", content: "What is the capital of France?" },
];

const first = WasmChatCompletionRequest.default();
first.model = "openai/gpt-4o";
first.messages = messages;
let response = await client.chat(first);
console.log(`Assistant: ${response.choices[0].message.content}`);

messages.push({ role: "assistant", content: response.choices[0].message.content! });
messages.push({ role: "user", content: "What about Germany?" });

const second = WasmChatCompletionRequest.default();
second.model = "openai/gpt-4o";
second.messages = messages;
response = await client.chat(second);
console.log(`Assistant: ${response.choices[0].message.content}`);
console.log(`Tokens: ${response.usage?.promptTokens} in, ${response.usage?.completionTokens} out`);
```
