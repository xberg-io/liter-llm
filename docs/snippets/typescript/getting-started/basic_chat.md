```typescript
import { createClient } from "@kreuzberg/liter-llm-node";

const client = createClient(process.env.OPENAI_API_KEY!);
const response = await client.chat({
  model: "openai/gpt-4o",
  messages: [{ role: "user", content: "Hello!" }],
});
console.log(response.choices[0].message.content);
```
