<!-- snippet:compile-only -->

```csharp
using LiterLlm;

using var client = LiterLlmLib.CreateClient(
    apiKey: Environment.GetEnvironmentVariable("BRAVE_API_KEY")!,
    baseUrl: null, timeoutSecs: null, maxRetries: null, modelHint: null);

var response = await client.Search(new SearchRequest
{
    Model = "brave/web-search",
    Query = "What is Rust programming language?",
    MaxResults = 5
});

foreach (var result in response.Results)
{
    Console.WriteLine($"{result.Title}: {result.Url}");
}
```
