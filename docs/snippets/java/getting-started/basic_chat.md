<!-- snippet:compile-only -->

```java
import dev.kreuzberg.literllm.*;
import java.util.List;

public class Main {
    public static void main(String[] args) throws Exception {
        try (var client = LiterLlm.createClient(System.getenv("OPENAI_API_KEY"))) {
            var request = ChatCompletionRequest.builder()
                .withModel("openai/gpt-4o")
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
