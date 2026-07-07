<!-- snippet:compile-only -->

```csharp
using LiterLlm;

using var client = LiterLlmLib.CreateClient(
    apiKey: Environment.GetEnvironmentVariable("OPENAI_API_KEY")!,
    baseUrl: null, timeoutSecs: null, maxRetries: null, modelHint: null);

var fileBytes = await File.ReadAllBytesAsync("data.jsonl");
var response = await client.CreateFile(new CreateFileRequest
{
    File = Convert.ToBase64String(fileBytes),
    Filename = "data.jsonl",
    Purpose = FilePurpose.Batch
});
Console.WriteLine($"File ID: {response.Id}");
Console.WriteLine($"Size: {response.Bytes} bytes");
```
