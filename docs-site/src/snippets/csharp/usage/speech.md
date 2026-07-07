<!-- snippet:compile-only -->

```csharp
using LiterLlm;

using var client = LiterLlmLib.CreateClient(
    apiKey: Environment.GetEnvironmentVariable("OPENAI_API_KEY")!,
    baseUrl: null, timeoutSecs: null, maxRetries: null, modelHint: null);

var audioBytes = await client.Speech(new CreateSpeechRequest
{
    Model = "openai/tts-1",
    Input = "Hello, world!",
    Voice = "alloy"
});
await File.WriteAllBytesAsync("output.mp3", audioBytes);
Console.WriteLine($"Wrote {audioBytes.Length} bytes to output.mp3");
```
