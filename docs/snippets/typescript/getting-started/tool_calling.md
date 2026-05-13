```typescript
import { createClient } from "@kreuzberg/liter-llm-node";

const client = createClient(process.env.OPENAI_API_KEY!);

const response = await client.chat({
  model: "openai/gpt-4o",
  messages: [{ role: "user", content: "What is the weather in Berlin?" }],
  toolChoice: "auto",
  tools: [
    {
      type: "function",
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
