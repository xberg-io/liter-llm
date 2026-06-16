<!-- snippet:compile-only -->

```csharp
using LiterLlm;

using var client = LiterLlmLib.CreateClient(
    apiKey: "sk-...",                       // or Environment.GetEnvironmentVariable("OPENAI_API_KEY")!
    baseUrl: "https://api.openai.com/v1",   // override provider base URL
    timeoutSecs: 60,                         // request timeout in seconds
    maxRetries: 3,                           // retry on transient failures
    modelHint: "openai"                      // pre-resolve provider at construction
);

var response = await client.ChatAsync(new ChatCompletionRequest
{
    Model = "openai/gpt-4o",
    Messages = [new Message.User(new UserMessage { Content = UserContent.Of("Hello!") })]
});
Console.WriteLine(response.Choices[0].Message.Content);
```
