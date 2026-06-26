<!-- snippet:compile-only -->

```typescript
import init, { createClient, WasmCreateTranscriptionRequest } from "@xberg-io/liter-llm-wasm";

await init();

const client = createClient(process.env.OPENAI_API_KEY!);

// `file` is a base64-encoded string, not raw bytes.
const audioBytes = new Uint8Array(/* read your audio file */);
const fileBase64 = btoa(String.fromCharCode(...audioBytes));

const request = WasmCreateTranscriptionRequest.default();
request.model = "openai/whisper-1";
request.file = fileBase64;

const response = await client.transcribe(request);
console.log(response.text);
```
