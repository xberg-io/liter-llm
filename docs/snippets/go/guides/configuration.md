```go
package main

import (
	"encoding/json"
	"fmt"
	"os"

	llm "github.com/xberg-io/liter-llm/packages/go"
)

func main() {
	// CreateClient signature:
	// CreateClient(apiKey string, baseURL *string, timeoutSecs *uint64,
	//              maxRetries *uint32, modelHint *string) (*DefaultClient, error)
	baseURL := "https://api.openai.com/v1"
	timeoutSecs := uint64(60)
	maxRetries := uint32(3)
	modelHint := "openai"

	client, err := llm.CreateClient(
		os.Getenv("OPENAI_API_KEY"),
		&baseURL,
		&timeoutSecs,
		&maxRetries,
		&modelHint,
	)
	if err != nil {
		panic(err)
	}

	var req llm.ChatCompletionRequest
	if err := json.Unmarshal([]byte(`{
		"model": "openai/gpt-4o-mini",
		"messages": [{"role": "user", "content": "Hello!"}]
	}`), &req); err != nil {
		panic(err)
	}

	resp, err := client.Chat(req)
	if err != nil {
		panic(err)
	}
	if len(resp.Choices) > 0 && resp.Choices[0].Message.Content != nil {
		fmt.Println(*resp.Choices[0].Message.Content)
	}
}
```
