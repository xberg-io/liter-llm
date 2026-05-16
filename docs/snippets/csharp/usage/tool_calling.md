<!-- snippet:compile-only -->

```csharp
using LiterLlm;

using var client = LiterLlmLib.CreateClient(
    apiKey: Environment.GetEnvironmentVariable("OPENAI_API_KEY")!,
    baseUrl: null, timeoutSecs: null, maxRetries: null, modelHint: null);

var tools = new List<ChatCompletionTool>
{
    new ChatCompletionTool
    {
        ToolType = ToolType.Function,
        Function = new FunctionDefinition
        {
            Name = "get_weather",
            Description = "Get the current weather for a location",
            Parameters = new
            {
                type = "object",
                properties = new
                {
                    location = new { type = "string", description = "City name" }
                },
                required = new[] { "location" }
            }
        }
    }
};

var response = await client.Chat(new ChatCompletionRequest
{
    Model = "openai/gpt-4o",
    Messages = [new Message.User(new UserMessage { Content = UserContent.Of("What is the weather in Berlin?") })],
    Tools = tools
});

foreach (var call in response.Choices[0].Message.ToolCalls ?? [])
{
    Console.WriteLine($"Tool: {call.Function.Name}, Args: {call.Function.Arguments}");
}
```
