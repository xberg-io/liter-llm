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

	body := `{
		"model": "openai/gpt-4o-mini",
		"messages": [{"role": "user", "content": "What is the weather in Berlin?"}],
		"tool_choice": "auto",
		"tools": [{
			"type": "function",
			"function": {
				"name": "get_weather",
				"description": "Get the current weather for a location",
				"parameters": {
					"type": "object",
					"properties": {"location": {"type": "string", "description": "City name"}},
					"required": ["location"]
				}
			}
		}]
	}`

	var req llm.ChatCompletionRequest
	if err := json.Unmarshal([]byte(body), &req); err != nil {
		panic(err)
	}

	resp, err := client.Chat(req)
	if err != nil {
		panic(err)
	}
	for _, call := range resp.Choices[0].Message.ToolCalls {
		fmt.Printf("Tool: %s, Args: %s\n", call.Function.Name, call.Function.Arguments)
	}
}
```
