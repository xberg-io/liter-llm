<!-- snippet:compile-only -->

```go
package main

import (
	"fmt"
	"os"

	llm "github.com/kreuzberg-dev/liter-llm/packages/go"
)

func main() {
	client, err := llm.CreateClient(os.Getenv("OPENAI_API_KEY"), nil, nil, nil, nil)
	if err != nil {
		panic(err)
	}

	req := llm.CreateBatchRequest{
		InputFileID:      "file-abc123",
		Endpoint:         "/v1/chat/completions",
		CompletionWindow: "24h",
	}

	resp, err := client.CreateBatch(req)
	if err != nil {
		panic(err)
	}
	fmt.Printf("Batch ID: %s\n", resp.ID)
	fmt.Printf("Status: %s\n", resp.Status)
}
```
