<!-- snippet:compile-only -->

```go
package main

import (
	"encoding/json"
	"fmt"
	"os"

	llm "github.com/kreuzberg-dev/liter-llm/packages/go"
)

func main() {
	client, err := llm.CreateClient(os.Getenv("MISTRAL_API_KEY"), nil, nil, nil, nil)
	if err != nil {
		panic(err)
	}

	var req llm.OcrRequest
	if err := json.Unmarshal([]byte(`{
		"model": "mistral/mistral-ocr-latest",
		"document": {"type": "document_url", "url": "https://example.com/invoice.pdf"}
	}`), &req); err != nil {
		panic(err)
	}

	resp, err := client.Ocr(req)
	if err != nil {
		panic(err)
	}
	for _, page := range resp.Pages {
		fmt.Printf("Page %d: %.100s...\n", page.Index, page.Markdown)
	}
}
```
