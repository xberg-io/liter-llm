<!-- snippet:compile-only -->

```typescript
import { createClient } from "@kreuzberg/liter-llm-node";
import { readFileSync } from "node:fs";

const client = createClient(process.env.OPENAI_API_KEY!);
// file is a base64-encoded string, not raw bytes.
const file = readFileSync("audio.mp3").toString("base64");
const response = await client.transcribe({
  model: "openai/whisper-1",
  file,
});
console.log(response.text);
```
