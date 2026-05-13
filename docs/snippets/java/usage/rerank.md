<!-- snippet:compile-only -->

```java
import dev.kreuzberg.literllm.*;
import java.util.List;

public class Main {
    public static void main(String[] args) throws Exception {
        try (var client = LiterLlm.createClient(System.getenv("OPENAI_API_KEY"))) {
            var docs = List.of(
                RerankDocument.of("Paris is the capital of France."),
                RerankDocument.of("Berlin is the capital of Germany."),
                RerankDocument.of("London is the capital of England.")
            );
            var response = client.rerank(RerankRequest.builder()
                .withModel("cohere/rerank-v3.5")
                .withQuery("What is the capital of France?")
                .withDocuments(docs)
                .build());
            for (var result : response.results()) {
                System.out.printf("Index: %d, Score: %.4f%n",
                    result.index(), result.relevanceScore());
            }
        }
    }
}
```
