<!-- snippet:compile-only -->

```typescript
import init, { createClient, WasmModerationRequest } from "@xberg-io/liter-llm-wasm";

await init();

const client = createClient(process.env.OPENAI_API_KEY!);

const request = WasmModerationRequest.default();
request.model = "openai/omni-moderation-latest";
request.input = "This is a test message.";

const response = await client.moderate(request);
const result = response.results[0];
console.log(`Flagged: ${result.flagged}`);
const cats = result.categories as Record<string, boolean | undefined>;
const scores = result.categoryScores as Record<string, number | undefined>;
for (const [category, flagged] of Object.entries(cats)) {
  if (flagged) {
    console.log(`  ${category}: ${(scores[category] ?? 0).toFixed(4)}`);
  }
}
```
