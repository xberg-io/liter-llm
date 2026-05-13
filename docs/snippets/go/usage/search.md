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
	client, err := llm.CreateClient(os.Getenv("BRAVE_API_KEY"), nil, nil, nil, nil)
	if err != nil {
		panic(err)
	}

	var req llm.SearchRequest
	if err := json.Unmarshal([]byte(`{
		"model": "brave/web-search",
		"query": "What is Rust programming language?",
		"max_results": 5
	}`), &req); err != nil {
		panic(err)
	}

	resp, err := client.Search(req)
	if err != nil {
		panic(err)
	}
	for _, r := range resp.Results {
		fmt.Printf("%s: %s\n", r.Title, r.URL)
	}
}
```
