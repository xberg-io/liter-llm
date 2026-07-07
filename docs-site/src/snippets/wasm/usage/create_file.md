<!-- snippet:compile-only -->

```typescript
import init, {
  createClient,
  WasmCreateFileRequest,
  WasmFilePurpose,
} from "@xberg-io/liter-llm-wasm";

await init();

const client = createClient(process.env.OPENAI_API_KEY!);

// `file` is a base64-encoded string, not raw bytes.
const fileBytes = new Uint8Array(/* read from your storage */);
const fileBase64 = btoa(String.fromCharCode(...fileBytes));

const request = WasmCreateFileRequest.default();
request.file = fileBase64;
request.filename = "data.jsonl";
request.purpose = WasmFilePurpose.Batch;

const response = await client.createFile(request);
console.log(`File ID: ${response.id}`);
console.log(`Size: ${response.bytes} bytes`);
```
