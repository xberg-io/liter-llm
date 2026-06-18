```go
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"os"

	ll "github.com/kreuzberg-dev/liter-llm/packages/go"
)

func main() {
	ctx := context.Background()

	// Create client with OpenAI API key
	client, err := ll.CreateClient(os.Getenv("OPENAI_API_KEY"), "", 0, 0, "")
	if err != nil {
		log.Fatalf("failed to create client: %v", err)
	}

	// Build a multimodal user message with text and image
	imageURL := ll.ImageURL{
		URL:    "https://upload.wikimedia.org/wikipedia/commons/thumb/d/dd/Gfp-wisconsin-madison-the-nature-boardwalk.jpg/2560px-Gfp-wisconsin-madison-the-nature-boardwalk.jpg",
		Detail: ll.Ptr(ll.ImageDetailLow),
	}

	userMessage := ll.Message{
		Role: "user",
		User: &ll.UserMessage{
			Content: ll.UserContent(json.RawMessage(`[
				{"type":"text","text":"Describe this image in one sentence."},
				{"type":"image_url","image_url":{"url":"https://upload.wikimedia.org/wikipedia/commons/thumb/d/dd/Gfp-wisconsin-madison-the-nature-boardwalk.jpg/2560px-Gfp-wisconsin-madison-the-nature-boardwalk.jpg","detail":"Low"}}
			]`)),
			Name: nil,
		},
	}

	// Create chat request with JSON schema response format
	schema := ll.JSONSchemaFormat{
		Name:        "ImageDescription",
		Description: ll.Ptr("A single-sentence description of an image"),
		Schema: json.RawMessage(`{
			"type": "object",
			"properties": {
				"description": {"type": "string"}
			},
			"required": ["description"]
		}`),
		Strict: ll.Ptr(true),
	}

	request := ll.ChatCompletionRequest{
		Model:    "openai/gpt-4o",
		Messages: []ll.Message{userMessage},
		ResponseFormat: ll.ResponseFormatJSONSchema{
			JSONSchema: schema,
		},
		Modalities: []ll.Modality{ll.ModalityText},
	}

	// Send request and get response
	response, err := client.Chat(ctx, request)
	if err != nil {
		log.Fatalf("failed to call chat: %v", err)
	}

	// Extract response text
	if len(response.Choices) > 0 {
		choice := response.Choices[0]
		if choice.Message.Content != nil {
			fmt.Printf("Response: %s\n", string(*choice.Message.Content))
		}
	}

	// Example: Request multimodal output (image + text)
	requestWithImage := ll.ChatCompletionRequest{
		Model:      "openai/gpt-4o",
		Messages:   []ll.Message{userMessage},
		Modalities: []ll.Modality{ll.ModalityText, ll.ModalityImage},
	}

	responseWithImage, err := client.Chat(ctx, requestWithImage)
	if err != nil {
		log.Fatalf("failed to call chat with image output: %v", err)
	}

	// Extract output images using helper method
	if len(responseWithImage.Choices) > 0 {
		choice := responseWithImage.Choices[0]
		outputImages, err := choice.Message.OutputImages()
		if err == nil {
			for i, img := range outputImages {
				fmt.Printf("Output image %d: %s\n", i, img.URL)
			}
		}
	}

	// Example: Stream multimodal response
	stream, err := client.ChatStream(ctx, requestWithImage)
	if err != nil {
		log.Fatalf("failed to start stream: %v", err)
	}
	defer stream.Close()

	for {
		chunk, err := stream.Next()
		if err != nil {
			log.Fatalf("stream error: %v", err)
		}
		if chunk == nil {
			break
		}
		if len(chunk.Choices) > 0 && chunk.Choices[0].Delta.Content != nil {
			fmt.Printf("Streamed: %s", string(*chunk.Choices[0].Delta.Content))
		}
	}
}
```
