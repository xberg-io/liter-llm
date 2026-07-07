<!-- snippet:compile-only -->

```csharp
using LiterLlm;

using var client = LiterLlmLib.CreateClient(
    apiKey: Environment.GetEnvironmentVariable("OPENAI_API_KEY")!,
    baseUrl: null, timeoutSecs: null, maxRetries: null, modelHint: null);

var response = await client.CreateBatch(new CreateBatchRequest
{
    InputFileId = "file-abc123",
    Endpoint = "/v1/chat/completions",
    CompletionWindow = "24h"
});
Console.WriteLine($"Batch ID: {response.Id}");
Console.WriteLine($"Status: {response.Status}");
```
