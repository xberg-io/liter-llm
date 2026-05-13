<!-- snippet:compile-only -->

```csharp
using LiterLlm;

using var client = LiterLlmLib.CreateClient(
    apiKey: Environment.GetEnvironmentVariable("OPENAI_API_KEY")!,
    baseUrl: null, timeoutSecs: null, maxRetries: null, modelHint: null);

var response = await client.Moderate(new ModerationRequest
{
    Model = "openai/omni-moderation-latest",
    Input = ModerationInput.Of("This is a test message.")
});

var result = response.Results[0];
var cats = result.Categories;
Console.WriteLine($"Flagged: {result.Flagged}");
// ModerationCategories is a typed class with bool fields (Sexual, Hate,
// Harassment, SelfHarm, Violence, ...); access them directly.
if (cats.Hate) Console.WriteLine("  hate flagged");
if (cats.Harassment) Console.WriteLine("  harassment flagged");
```
