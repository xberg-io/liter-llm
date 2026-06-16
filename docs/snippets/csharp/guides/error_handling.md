<!-- snippet:compile-only -->

```csharp
using LiterLlm;

using var client = LiterLlmLib.CreateClient(
    apiKey: Environment.GetEnvironmentVariable("OPENAI_API_KEY")!,
    baseUrl: null, timeoutSecs: null, maxRetries: null, modelHint: null);

try
{
    var response = await client.ChatAsync(new ChatCompletionRequest
    {
        Model = "openai/gpt-4o",
        Messages = [new Message.User(new UserMessage { Content = UserContent.Of("Hello") })]
    });
    Console.WriteLine(response.Choices[0].Message.Content);
}
catch (AuthenticationException e)
{
    // 401/403 — rotate the key.
    Console.Error.WriteLine($"auth failed: {e.Message}");
}
catch (RateLimitedException e)
{
    // 429 — transient, retry with backoff.
    Console.Error.WriteLine($"rate limited: {e.Message}");
}
catch (BudgetExceededException e)
{
    Console.Error.WriteLine($"budget exceeded: {e.Message}");
}
catch (ServerErrorException e)
{
    // 5xx — usually transient.
    Console.Error.WriteLine($"server error: {e.Message}");
}
catch (EndpointNotSupportedException e)
{
    Console.Error.WriteLine($"endpoint not supported by provider: {e.Message}");
}
catch (LiterLlmErrorException e)
{
    // Catch-all for typed liter-llm errors.
    Console.Error.WriteLine($"llm error: {e.Message}");
}
catch (LiterLlmException e)
{
    // FFI-level error (carries a numeric code).
    Console.Error.WriteLine($"ffi error ({e.Code}): {e.Message}");
}
```
