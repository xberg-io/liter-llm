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

	var req llm.CreateSpeechRequest
	if err := json.Unmarshal([]byte(`{
		"model": "openai/tts-1",
		"input": "Hello, world!",
		"voice": "alloy"
	}`), &req); err != nil {
		panic(err)
	}

	audio, err := client.Speech(req)
	if err != nil {
		panic(err)
	}
	if err := os.WriteFile("output.mp3", audio, 0o644); err != nil {
		panic(err)
	}
	fmt.Printf("Wrote %d bytes to output.mp3\n", len(audio))
}
```
