<!-- snippet:compile-only -->

```csharp
using LiterLlm;

using var client = LiterLlmLib.CreateClient(
    apiKey: Environment.GetEnvironmentVariable("OPENAI_API_KEY")!,
    baseUrl: null, timeoutSecs: null, maxRetries: null, modelHint: null);

var audioBytes = await File.ReadAllBytesAsync("audio.mp3");
var response = await client.Transcribe(new CreateTranscriptionRequest
{
    Model = "openai/whisper-1",
    File = Convert.ToBase64String(audioBytes)
});
Console.WriteLine(response.Text);
```
