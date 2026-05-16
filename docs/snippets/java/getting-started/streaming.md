<!-- snippet:compile-only -->

```java
import dev.kreuzberg.literllm.*;
import java.util.List;

public class Main {
    public static void main(String[] args) throws Exception {
        try (var client = LiterLlm.createClient(System.getenv("OPENAI_API_KEY"))) {
            var request = ChatCompletionRequest.builder()
                .withModel("openai/gpt-4o-mini")
                .withMessages(List.of(
                    new Message.User(new UserMessage(UserContent.of("Hello"), null))
                ))
                .build();
            var stream = client.chatStream(request);
            while (stream.hasNext()) {
                var chunk = stream.next();
                var delta = chunk.choices().getFirst().delta().content();
                if (delta != null) System.out.print(delta);
            }
            System.out.println();
        }
    }
}
```
