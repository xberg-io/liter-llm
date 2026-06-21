<!-- snippet:compile-only -->

```typescript
import { createClient, ToolType } from "@kreuzberg/liter-llm";

const client = createClient(process.env.OPENAI_API_KEY!);

const response = await client.chat({
  model: "openai/gpt-4o",
  messages: [{ role: "user", content: "What is the weather in Berlin?" }],
  tools: [
    {
      toolType: ToolType.Function,
      function: {
        name: "get_weather",
        description: "Get the current weather for a location",
        parameters: {
          type: "object",
          properties: { location: { type: "string" } },
          required: ["location"],
        },
      },
    },
  ],
});

for (const call of response.choices[0]?.message?.toolCalls ?? []) {
  console.log(`Tool: ${call.function.name}, Args: ${call.function.arguments}`);
}
```
