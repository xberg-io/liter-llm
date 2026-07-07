<!-- snippet:compile-only -->

```java
import io.xberg.literllm.*;

public class Main {
    public static void main(String[] args) throws Exception {
        try (var client = LiterLlm.createClient(System.getenv("OPENAI_API_KEY"))) {
            var response = client.createResponse(CreateResponseRequest.builder()
                .withModel("openai/gpt-4o")
                .withInput("Explain quantum computing in one sentence.")
                .build());
            System.out.println(response);
        }
    }
}
```
