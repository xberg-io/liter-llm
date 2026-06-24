<!-- snippet:compile-only -->

```go
package main

import (
	"encoding/json"
	"fmt"
	"os"

	llm "github.com/xberg-io/liter-llm/packages/go"
)

func main() {
	client, err := llm.CreateClient(os.Getenv("COHERE_API_KEY"), nil, nil, nil, nil)
	if err != nil {
		panic(err)
	}

	var req llm.RerankRequest
	if err := json.Unmarshal([]byte(`{
		"model": "cohere/rerank-v3.5",
		"query": "What is the capital of France?",
		"documents": [
			"Paris is the capital of France.",
			"Berlin is the capital of Germany.",
			"London is the capital of England."
		]
	}`), &req); err != nil {
		panic(err)
	}

	resp, err := client.Rerank(req)
	if err != nil {
		panic(err)
	}
	for _, r := range resp.Results {
		fmt.Printf("Index: %d, Score: %.4f\n", r.Index, r.RelevanceScore)
	}
}
```
