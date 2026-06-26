<!-- snippet:compile-only -->

```typescript
import init, { createClient, WasmCreateSpeechRequest } from "@xberg-io/liter-llm-wasm";

await init();

const client = createClient(process.env.OPENAI_API_KEY!);

const request = WasmCreateSpeechRequest.default();
request.model = "openai/tts-1";
request.input = "Hello, world!";
request.voice = "alloy";

const audio = await client.speech(request);
console.log(`Generated ${audio.byteLength} bytes of audio`);
```
