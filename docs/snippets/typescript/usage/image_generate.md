<!-- snippet:compile-only -->

```typescript
import { createClient } from "@xberg-io/liter-llm";

const client = createClient(process.env.OPENAI_API_KEY!);
const response = await client.imageGenerate({
  model: "openai/dall-e-3",
  prompt: "A sunset over mountains",
  n: 1,
  size: "1024x1024",
});
console.log(response.data?.[0]?.url);
```
