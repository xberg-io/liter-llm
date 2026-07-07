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
		"messages": [{"role": "user", "content": "Explain quantum computing briefly"}],
		"stream": true
	}`), &req); err != nil {
		panic(err)
	}

	stream, err := client.ChatStream(req)
	if err != nil {
		panic(err)
	}

	var sb strings.Builder
	for chunk := range stream {
		if len(chunk.Choices) > 0 && chunk.Choices[0].Delta.Content != nil {
			delta := *chunk.Choices[0].Delta.Content
			sb.WriteString(delta)
			fmt.Print(delta)
		}
	}
	fmt.Println()
	fmt.Printf("Full response length: %d characters\n", sb.Len())
}
```
