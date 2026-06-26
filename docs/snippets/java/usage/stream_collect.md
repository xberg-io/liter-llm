<!-- snippet:compile-only -->

```java
import io.xberg.literllm.*;
import java.util.List;

public class Main {
    public static void main(String[] args) throws Exception {
        try (var client = LiterLlm.createClient(System.getenv("OPENAI_API_KEY"))) {
            var request = ChatCompletionRequest.builder()
                .withModel("openai/gpt-4o")
                .withMessages(List.of(
                    new Message.User(new UserMessage(UserContent.of("Explain quantum computing briefly"), null))
                ))
                .build();
            var sb = new StringBuilder();
            var stream = client.chatStream(request);
            var iterator = stream.iterator();
            while (iterator.hasNext()) {
                var chunk = iterator.next();
                var delta = chunk.choices().getFirst().delta().content();
                if (delta != null) {
                    sb.append(delta);
                    System.out.print(delta);
                }
            }
            System.out.println();
            System.out.printf("%nFull response length: %d characters%n", sb.length());
        }
    }
}
```
