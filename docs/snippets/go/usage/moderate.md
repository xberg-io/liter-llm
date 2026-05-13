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
	client, err := llm.CreateClient(os.Getenv("OPENAI_API_KEY"), nil, nil, nil, nil)
	if err != nil {
		panic(err)
	}

	var req llm.ModerationRequest
	if err := json.Unmarshal([]byte(`{
		"model": "openai/omni-moderation-latest",
		"input": "This is a test message."
	}`), &req); err != nil {
		panic(err)
	}

	resp, err := client.Moderate(req)
	if err != nil {
		panic(err)
	}
	first := resp.Results[0]
	fmt.Printf("Flagged: %v\n", first.Flagged)
}
```
