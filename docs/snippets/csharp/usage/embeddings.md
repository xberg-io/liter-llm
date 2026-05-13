```csharp
using LiterLlm;

using var client = LiterLlmLib.CreateClient(
    apiKey: Environment.GetEnvironmentVariable("OPENAI_API_KEY")!,
    baseUrl: null, timeoutSecs: null, maxRetries: null, modelHint: null);

var response = await client.Embed(new EmbeddingRequest
{
    Model = "openai/text-embedding-3-small",
    Input = EmbeddingInput.Of(new[] { "The quick brown fox jumps over the lazy dog" })
});

var embedding = response.Data[0].Embedding;
Console.WriteLine($"Dimensions: {embedding.Count}");
Console.WriteLine($"First 5 values: [{string.Join(", ", embedding.Take(5))}]");
```
