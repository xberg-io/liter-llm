<!-- snippet:compile-only -->

```java
import io.xberg.literllm.*;
import java.util.Optional;

public class Main {
    public static void main(String[] args) throws Exception {
        try (var client = LiterLlm.createClient(System.getenv("BRAVE_API_KEY"))) {
            var response = client.search(SearchRequest.builder()
                .withModel("brave/web-search")
                .withQuery("What is Rust programming language?")
                .withMaxResults(Optional.of(5))
                .build());
            for (var result : response.results()) {
                System.out.printf("%s: %s%n", result.title(), result.url());
            }
        }
    }
}
```
