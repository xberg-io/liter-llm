```go
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"

	llm "github.com/xberg-io/liter-llm/packages/go"
)

func main() {
	client, err := llm.CreateClient(os.Getenv("OPENAI_API_KEY"), nil, nil, nil, nil)
	if err != nil {
		panic(err)
	}

	var req llm.ChatCompletionRequest
	if err := json.Unmarshal([]byte(`{
		"model": "openai/gpt-4o-mini",
		"messages": [{"role": "user", "content": "Hello"}]
	}`), &req); err != nil {
		panic(err)
	}

	_, err = client.Chat(req)
	if err == nil {
		return
	}

	// Errors from the Go binding are plain `error` values formatted as
	// "[<code>] <message>". Match on the message text to identify the
	// category until a structured error type is exposed.
	msg := err.Error()
	switch {
	case strings.Contains(msg, "authentication"):
		fmt.Println("auth failed:", err)
	case strings.Contains(msg, "rate limit"):
		fmt.Println("rate limited:", err)
	case strings.Contains(msg, "context window"):
		fmt.Println("prompt too long:", err)
	case strings.Contains(msg, "service unavailable"):
		fmt.Println("provider unavailable:", err)
	default:
		fmt.Println("llm error:", err)
	}
}
```
