<!-- snippet:compile-only -->

```java
import dev.kreuzberg.literllm.*;
import java.nio.file.Files;
import java.nio.file.Path;

public class Main {
    public static void main(String[] args) throws Exception {
        try (var client = LiterLlm.createClient(System.getenv("OPENAI_API_KEY"))) {
            byte[] audioBytes = client.speech(CreateSpeechRequest.builder()
                .withModel("openai/tts-1")
                .withInput("Hello, world!")
                .withVoice("alloy")
                .build());
            Files.write(Path.of("output.mp3"), audioBytes);
            System.out.printf("Wrote %d bytes to output.mp3%n", audioBytes.length);
        }
    }
}
```
