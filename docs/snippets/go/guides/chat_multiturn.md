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

	body := map[string]any{
		"model": "openai/gpt-4o-mini",
		"messages": []map[string]string{
			{"role": "system", "content": "You are a helpful assistant."},
			{"role": "user", "content": "What is the capital of France?"},
		},
	}

	var req llm.ChatCompletionRequest
	raw, _ := json.Marshal(body)
	_ = json.Unmarshal(raw, &req)
	resp, err := client.Chat(req)
	if err != nil {
		panic(err)
	}
	answer := ""
	if len(resp.Choices) > 0 && resp.Choices[0].Message.Content != nil {
		answer = *resp.Choices[0].Message.Content
	}
	fmt.Printf("Assistant: %s\n", answer)

	body["messages"] = append(body["messages"].([]map[string]string),
		map[string]string{"role": "assistant", "content": answer},
		map[string]string{"role": "user", "content": "What about Germany?"},
	)
	raw, _ = json.Marshal(body)
	_ = json.Unmarshal(raw, &req)
	resp, err = client.Chat(req)
	if err != nil {
		panic(err)
	}
	if len(resp.Choices) > 0 && resp.Choices[0].Message.Content != nil {
		fmt.Printf("Assistant: %s\n", *resp.Choices[0].Message.Content)
	}
	if resp.Usage != nil {
		fmt.Printf("Tokens: %d in, %d out\n", resp.Usage.PromptTokens, resp.Usage.CompletionTokens)
	}
}
```
