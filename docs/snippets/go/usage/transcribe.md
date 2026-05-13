<!-- snippet:compile-only -->

```go
package main

import (
	"encoding/base64"
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

	audio, err := os.ReadFile("audio.mp3")
	if err != nil {
		panic(err)
	}

	body, _ := json.Marshal(map[string]string{
		"model": "openai/whisper-1",
		"file":  base64.StdEncoding.EncodeToString(audio),
	})

	var req llm.CreateTranscriptionRequest
	if err := json.Unmarshal(body, &req); err != nil {
		panic(err)
	}

	resp, err := client.Transcribe(req)
	if err != nil {
		panic(err)
	}
	fmt.Println(resp.Text)
}
```
