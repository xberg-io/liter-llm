```typescript
import init, {
  createClient,
  WasmChatCompletionRequest,
  WasmChatCompletionTool,
  WasmFunctionDefinition,
} from "@kreuzberg/liter-llm-wasm";

await init();

const client = createClient(process.env.OPENAI_API_KEY!);

const tool = WasmChatCompletionTool.default();
tool.type = "function";
const fn = WasmFunctionDefinition.default();
fn.name = "get_weather";
fn.description = "Get the current weather for a location";
fn.parameters = {
  type: "object",
  properties: { location: { type: "string" } },
  required: ["location"],
};
tool.function = fn;

const request = WasmChatCompletionRequest.default();
request.model = "openai/gpt-4o";
request.messages = [{ role: "user", content: "What is the weather in Berlin?" }];
request.toolChoice = "auto";
request.tools = [tool];

const response = await client.chat(request);
for (const call of response.choices[0]?.message?.toolCalls ?? []) {
  console.log(`Tool: ${call.function.name}, Args: ${call.function.arguments}`);
}
```
