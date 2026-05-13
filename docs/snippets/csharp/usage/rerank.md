<!-- snippet:compile-only -->

```csharp
using LiterLlm;

using var client = LiterLlmLib.CreateClient(
    apiKey: Environment.GetEnvironmentVariable("OPENAI_API_KEY")!,
    baseUrl: null, timeoutSecs: null, maxRetries: null, modelHint: null);

var response = await client.Rerank(new RerankRequest
{
    Model = "cohere/rerank-v3.5",
    Query = "What is the capital of France?",
    Documents =
    [
        RerankDocument.Of("Paris is the capital of France."),
        RerankDocument.Of("Berlin is the capital of Germany."),
        RerankDocument.Of("London is the capital of England."),
    ]
});

foreach (var result in response.Results)
{
    Console.WriteLine($"Index: {result.Index}, Score: {result.RelevanceScore:F4}");
}
```
