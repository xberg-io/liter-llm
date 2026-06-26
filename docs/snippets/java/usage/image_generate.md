<!-- snippet:compile-only -->

```java
import io.xberg.literllm.*;
import java.util.Optional;

public class Main {
    public static void main(String[] args) throws Exception {
        try (var client = LiterLlm.createClient(System.getenv("OPENAI_API_KEY"))) {
            var response = client.imageGenerate(CreateImageRequest.builder()
                .withPrompt("A sunset over mountains")
                .withModel(Optional.of("openai/dall-e-3"))
                .withN(Optional.of(1))
                .withSize(Optional.of("1024x1024"))
                .build());
            System.out.println(response.data().getFirst().url());
        }
    }
}
```
