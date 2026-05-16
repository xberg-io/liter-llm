<!-- snippet:compile-only -->

```csharp
using LiterLlm;

using var client = LiterLlmLib.CreateClient(
    apiKey: Environment.GetEnvironmentVariable("OPENAI_API_KEY")!,
    baseUrl: null, timeoutSecs: null, maxRetries: null, modelHint: null);

var request = new ChatCompletionRequest
{
    Model = "openai/gpt-4o-mini",
    Messages = [new Message.User(new UserMessage { Content = UserContent.Of("Hello") })]
};

await foreach (var chunk in client.ChatStream(request))
{
    var delta = chunk.Choices.Count > 0 ? chunk.Choices[0].Delta.Content : null;
    if (delta is not null) Console.Write(delta);
}
Console.WriteLine();
```
