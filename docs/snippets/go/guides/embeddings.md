```go
package main

import (
	"encoding/json"
	"fmt"
	"os"

	llm "github.com/kreuzberg-dev/liter-llm/packages/go"
)

func main() {
	client, err := llm.CreateClient(os.Getenv("OPENAI_API_KEY"), nil, nil, nil, nil)
	if err != nil {
		panic(err)
	}

	var req llm.EmbeddingRequest
	if err := json.Unmarshal([]byte(`{
		"model": "openai/text-embedding-3-small",
		"input": ["The quick brown fox jumps over the lazy dog"]
	}`), &req); err != nil {
		panic(err)
	}

	resp, err := client.Embed(req)
	if err != nil {
		panic(err)
	}
	fmt.Printf("Dimensions: %d\n", len(resp.Data[0].Embedding))
	fmt.Printf("First 5 values: %v\n", resp.Data[0].Embedding[:5])
}
```
