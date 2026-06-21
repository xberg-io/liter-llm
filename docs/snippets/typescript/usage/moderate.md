<!-- snippet:compile-only -->

```typescript
import { createClient } from "@kreuzberg/liter-llm";

const client = createClient(process.env.OPENAI_API_KEY!);
const response = await client.moderate({
  model: "openai/omni-moderation-latest",
  input: "This is a test message.",
});

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
