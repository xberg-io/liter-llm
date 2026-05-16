<!-- snippet:compile-only -->

```java
import dev.kreuzberg.literllm.*;
import java.util.List;

public class LocalLlm {
    public static void main(String[] args) throws Exception {
        // No API key needed for local providers
        try (var client = LiterLlm.createClient("", "http://localhost:11434/v1")) {
            var request = ChatCompletionRequest.builder()
                .withModel("ollama/qwen2:0.5b")
                .withMessages(List.of(
                    new Message.User(new UserMessage(UserContent.of("Hello!"), null))
                ))
                .build();
            var response = client.chat(request);
            System.out.println(response.choices().getFirst().message().content());
        }
    }
}
```
