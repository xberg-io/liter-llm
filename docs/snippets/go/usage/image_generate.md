<!-- snippet:compile-only -->

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

	var req llm.CreateImageRequest
	if err := json.Unmarshal([]byte(`{
		"model": "openai/dall-e-3",
		"prompt": "A sunset over mountains",
		"n": 1,
		"size": "1024x1024"
	}`), &req); err != nil {
		panic(err)
	}

	resp, err := client.ImageGenerate(req)
	if err != nil {
		panic(err)
	}
	if len(resp.Data) > 0 && resp.Data[0].URL != nil {
		fmt.Println(*resp.Data[0].URL)
	}
}
```
