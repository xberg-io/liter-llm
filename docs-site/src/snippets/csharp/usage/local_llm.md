<!-- snippet:compile-only -->

```csharp
using LiterLlm;

using var client = LiterLlmLib.CreateClient(
    apiKey: "",
    baseUrl: "http://localhost:11434/v1",
    timeoutSecs: null,
    maxRetries: null,
    modelHint: "ollama/qwen2:0.5b");

var response = await client.ChatAsync(new ChatCompletionRequest
{
    Model = "ollama/qwen2:0.5b",
    Messages = new[]
    {
        new Message
        {
            Role = MessageRoleEnum.User,
            Content = "Hello!"
        }
    }
});

Console.WriteLine(response.Choices[0].Message.Content);
```
