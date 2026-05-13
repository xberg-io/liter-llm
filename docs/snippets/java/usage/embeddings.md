```java
import dev.kreuzberg.literllm.*;
import java.util.List;

public class Main {
    public static void main(String[] args) throws Exception {
        try (var client = LiterLlm.createClient(System.getenv("OPENAI_API_KEY"))) {
            var response = client.embed(EmbeddingRequest.builder()
                .withModel("openai/text-embedding-3-small")
                .withInput(EmbeddingInput.of(List.of("The quick brown fox jumps over the lazy dog")))
                .build());
            var embedding = response.data().getFirst().embedding();
            System.out.println("Dimensions: " + embedding.size());
            System.out.println("First 5 values: " + embedding.subList(0, 5));
        }
    }
}
```
