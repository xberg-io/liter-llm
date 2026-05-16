<!-- snippet:compile-only -->

```csharp
using LiterLlm;

using var client = LiterLlmLib.CreateClient(
    apiKey: Environment.GetEnvironmentVariable("OPENAI_API_KEY")!,
    baseUrl: null, timeoutSecs: null, maxRetries: null, modelHint: null);

var response = await client.Chat(new ChatCompletionRequest
{
    Model = "openai/gpt-4o",
    Messages = [new Message.User(new UserMessage { Content = UserContent.Of("Hello!") })]
});
Console.WriteLine(response.Choices[0].Message.Content);
```
