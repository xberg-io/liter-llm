<!-- snippet:compile-only -->

```java
import dev.kreuzberg.literllm.*;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Base64;

public class Main {
    public static void main(String[] args) throws Exception {
        try (var client = LiterLlm.createClient(System.getenv("OPENAI_API_KEY"))) {
            byte[] audioBytes = Files.readAllBytes(Path.of("audio.mp3"));
            String audioBase64 = Base64.getEncoder().encodeToString(audioBytes);
            var response = client.transcribe(CreateTranscriptionRequest.builder()
                .withModel("openai/whisper-1")
                .withFile(audioBase64)
                .build());
            System.out.println(response.text());
        }
    }
}
```
