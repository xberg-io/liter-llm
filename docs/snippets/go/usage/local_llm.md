```go
package main

import (
	"encoding/json"
	"fmt"

	llm "github.com/kreuzberg-dev/liter-llm/packages/go"
)

func main() {
	// Local providers (Ollama, LM Studio, ...) don't require an API key,
	// but a placeholder value is still required by the binding.
	baseURL := "http://localhost:11434/v1"
	client, err := llm.CreateClient("not-needed", &baseURL, nil, nil, nil)
	if err != nil {
		panic(err)
	}

	var req llm.ChatCompletionRequest
	if err := json.Unmarshal([]byte(`{
		"model": "ollama/qwen2:0.5b",
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
