<!-- snippet:compile-only -->

```go
package main

import (
	"encoding/base64"
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

	data, err := os.ReadFile("data.jsonl")
	if err != nil {
		panic(err)
	}

	body, _ := json.Marshal(map[string]string{
		"file":     base64.StdEncoding.EncodeToString(data),
		"filename": "data.jsonl",
		"purpose":  "batch",
	})

	var req llm.CreateFileRequest
	if err := json.Unmarshal(body, &req); err != nil {
		panic(err)
	}

	resp, err := client.CreateFile(req)
	if err != nil {
		panic(err)
	}
	fmt.Printf("File ID: %s\n", resp.ID)
	fmt.Printf("Size: %d bytes\n", resp.Bytes)
}
```
