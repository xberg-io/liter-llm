<!-- snippet:compile-only -->

```csharp
using System.Text;
using LiterLlm;

using var client = LiterLlmLib.CreateClient(
    apiKey: Environment.GetEnvironmentVariable("OPENAI_API_KEY")!,
    baseUrl: null, timeoutSecs: null, maxRetries: null, modelHint: null);

var request = new ChatCompletionRequest
{
    Model = "openai/gpt-4o",
    Messages = [new Message.User(new UserMessage { Content = UserContent.Of("Explain quantum computing briefly") })]
};

var sb = new StringBuilder();
await foreach (var chunk in client.ChatStreamAsync(request))
{
    var delta = chunk.Choices.Count > 0 ? chunk.Choices[0].Delta.Content : null;
    if (delta is not null)
    {
        sb.Append(delta);
        Console.Write(delta);
    }
}
Console.WriteLine();
Console.WriteLine($"\nFull response length: {sb.Length} characters");
```
