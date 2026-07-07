<!-- snippet:compile-only -->

```java
import io.xberg.literllm.*;
import java.util.List;

public class Main {
    public static void main(String[] args) throws Exception {
        // Positional args: apiKey, baseUrl, timeoutSecs, maxRetries, modelHint.
        // Pass null for any optional value to use the built-in default.
        try (var client = LiterLlm.createClient(
                "sk-...",                          // or System.getenv("OPENAI_API_KEY")
                "https://api.openai.com/v1",       // override provider base URL
                60L,                                // request timeout in seconds
                3,                                  // retry on transient failures
                "openai"                            // pre-resolve provider at construction
        )) {
            var response = client.chat(ChatCompletionRequest.builder()
                .withModel("openai/gpt-4o")
                .withMessages(List.of(
                    new Message.User(new UserMessage(UserContent.of("Hello!"), null))
                ))
                .build());
            System.out.println(response.choices().getFirst().message().content());
        }
    }
}
```
