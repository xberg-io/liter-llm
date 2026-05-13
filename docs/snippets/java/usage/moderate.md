<!-- snippet:compile-only -->

```java
import dev.kreuzberg.literllm.*;
import java.util.Optional;

public class Main {
    public static void main(String[] args) throws Exception {
        try (var client = LiterLlm.createClient(System.getenv("OPENAI_API_KEY"))) {
            var response = client.moderate(ModerationRequest.builder()
                .withInput(ModerationInput.of("This is a test message."))
                .withModel(Optional.of("openai/omni-moderation-latest"))
                .build());
            var result = response.results().getFirst();
            var cats = result.categories();
            System.out.println("Flagged: " + result.flagged());
            // ModerationCategories is a typed record with boolean fields (sexual,
            // hate, harassment, selfHarm, violence, ...); access them directly.
            if (cats.hate()) System.out.println("  hate flagged");
            if (cats.harassment()) System.out.println("  harassment flagged");
        }
    }
}
```
