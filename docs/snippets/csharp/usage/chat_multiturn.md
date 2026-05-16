<!-- snippet:compile-only -->

```csharp
using LiterLlm;

using var client = LiterLlmLib.CreateClient(
    apiKey: Environment.GetEnvironmentVariable("OPENAI_API_KEY")!,
    baseUrl: null, timeoutSecs: null, maxRetries: null, modelHint: null);

var messages = new List<Message>
{
    new Message.System(new SystemMessage { Content = "You are a helpful assistant." }),
    new Message.User(new UserMessage { Content = UserContent.Of("What is the capital of France?") }),
};

var response = await client.Chat(new ChatCompletionRequest { Model = "openai/gpt-4o", Messages = messages });
var content = response.Choices[0].Message.Content;
Console.WriteLine($"Assistant: {content}");

messages.Add(new Message.Assistant(new AssistantMessage { Content = content }));
messages.Add(new Message.User(new UserMessage { Content = UserContent.Of("What about Germany?") }));

response = await client.Chat(new ChatCompletionRequest { Model = "openai/gpt-4o", Messages = messages });
Console.WriteLine($"Assistant: {response.Choices[0].Message.Content}");

if (response.Usage is not null)
{
    Console.WriteLine($"Tokens: {response.Usage.PromptTokens} in, {response.Usage.CompletionTokens} out");
}
```
