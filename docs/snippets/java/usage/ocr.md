<!-- snippet:compile-only -->

```java
import dev.kreuzberg.literllm.*;

public class Main {
    public static void main(String[] args) throws Exception {
        try (var client = LiterLlm.createClient(System.getenv("MISTRAL_API_KEY"))) {
            var response = client.ocr(OcrRequest.builder()
                .withModel("mistral/mistral-ocr-latest")
                .withDocument(new OcrDocument.Url("https://example.com/invoice.pdf"))
                .build());
            for (var page : response.pages()) {
                System.out.printf("Page %d: %.100s...%n",
                    page.index(), page.markdown());
            }
        }
    }
}
```
