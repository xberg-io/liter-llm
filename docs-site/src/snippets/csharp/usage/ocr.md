<!-- snippet:compile-only -->

```csharp
using LiterLlm;

using var client = LiterLlmLib.CreateClient(
    apiKey: Environment.GetEnvironmentVariable("MISTRAL_API_KEY")!,
    baseUrl: null, timeoutSecs: null, maxRetries: null, modelHint: null);

var response = await client.Ocr(new OcrRequest
{
    Model = "mistral/mistral-ocr-latest",
    Document = new OcrDocument.Url("https://example.com/invoice.pdf")
});

foreach (var page in response.Pages)
{
    var preview = page.Markdown.Length > 100 ? page.Markdown[..100] : page.Markdown;
    Console.WriteLine($"Page {page.Index}: {preview}...");
}
```
