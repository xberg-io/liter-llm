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
	client, err := llm.CreateClient(os.Getenv("OPENAI_API_KEY"), nil, nil, nil, nil)
	if err != nil {
		panic(err)
	}

	req := llm.CreateResponseRequest{
		Model: "openai/gpt-4o",
		Input: json.RawMessage(`"Explain quantum computing in one sentence."`),
	}

	resp, err := client.CreateResponse(req)
	if err != nil {
		panic(err)
	}
	fmt.Printf("Response ID: %s\n", resp.ID)
	fmt.Printf("Status: %s\n", resp.Status)
	for _, item := range resp.Output {
		fmt.Println(string(item.Content))
	}
}
```
