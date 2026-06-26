---
description: "Vision input, structured JSON output, and multimodal output (images, audio) with liter-llm."
---

# Multimodal I/O

Send images and documents alongside text, request structured JSON responses, and receive images and audio from models.

## Vision Input

Send images to vision-capable models as remote URLs or base64 data URLs.

### Remote Image URL

=== "Python"

    ```python
    from liter_llm import create_client, ContentPart, ImageDetail

    client = create_client(api_key="sk-...")
    response = client.chat(
        model="gpt-4o",
        messages=[
            {
                "role": "user",
                "content": [
                    {"type": "text", "text": "What is in this image?"},
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": "https://example.com/image.jpg",
                            "detail": "high"  # low, high, auto
                        }
                    }
                ]
            }
        ]
    )
    print(response.choices[0].message.text())
    ```

=== "TypeScript"

    ```typescript
    import { createClient, ImageDetail } from "@xberg-io/liter-llm";

    const client = createClient({ apiKey: "sk-..." });
    const response = await client.chat({
        model: "gpt-4o",
        messages: [
            {
                role: "user",
                content: [
                    { type: "text", text: "What is in this image?" },
                    {
                        type: "image_url",
                        image_url: {
                            url: "https://example.com/image.jpg",
                            detail: "high"
                        }
                    }
                ]
            }
        ]
    });
    console.log(response.choices[0].message.text());
    ```

=== "Rust"

    ```rust
    use liter_llm::{create_client, ContentPart, ImageDetail};

    let client = create_client("sk-...", None, None, None, None)?;
    let response = client.chat(
        &liter_llm::ChatCompletionRequest {
            model: "gpt-4o".into(),
            messages: vec![
                liter_llm::Message::User(liter_llm::UserMessage {
                    content: liter_llm::UserContent::Parts(vec![
                        ContentPart::Text { text: "What is in this image?".into() },
                        ContentPart::ImageUrl {
                            image_url: liter_llm::ImageUrl {
                                url: "https://example.com/image.jpg".into(),
                                detail: Some(ImageDetail::High),
                            }
                        }
                    ]),
                    name: None,
                })
            ],
            ..Default::default()
        }
    ).await?;
    println!("{:?}", response.choices[0].message.text());
    ```

=== "Go"

    ```go
    package main

    import (
        "fmt"
        llm "liter-llm"
    )

    func main() {
        client, err := llm.CreateClient("sk-...", nil, nil, nil, nil)
        if err != nil {
            panic(err)
        }

        response, err := client.Chat(&llm.ChatCompletionRequest{
            Model: "gpt-4o",
            Messages: []llm.Message{
                &llm.UserMessage{
                    Content: llm.NewUserContentParts([]llm.ContentPart{
                        llm.NewContentPartText("What is in this image?"),
                        llm.NewContentPartImageUrl(llm.ImageUrl{
                            URL:    "https://example.com/image.jpg",
                            Detail: llm.ImageDetailHigh,
                        }),
                    }),
                },
            },
        })
        if err != nil {
            panic(err)
        }
        fmt.Println(response.Choices[0].Message.Text())
    }
    ```

=== "Java"

    ```java
    import io.xberg.literllm.*;

    DefaultClient client = LiterLlm.createClient("sk-...", null, null, null, null);

    ChatCompletionRequest req = new ChatCompletionRequest()
        .model("gpt-4o")
        .messages(Arrays.asList(
            new UserMessage()
                .content(new UserContent(Arrays.asList(
                    ContentPart.text("What is in this image?"),
                    ContentPart.imageUrl(new ImageUrl()
                        .url("https://example.com/image.jpg")
                        .detail(ImageDetail.HIGH))
                )))
        ));

    ChatCompletionResponse resp = client.chat(req);
    System.out.println(resp.getChoices().get(0).getMessage().text());
    ```

=== "C#"

    ```csharp
    using LiterLlm;

    var client = LiterLlm.CreateClient("sk-...", null, null, null, null);

    var response = await client.Chat(new ChatCompletionRequest
    {
        Model = "gpt-4o",
        Messages = new List<Message>
        {
            new UserMessage
            {
                Content = new UserContent(new List<ContentPart>
                {
                    ContentPart.Text("What is in this image?"),
                    ContentPart.ImageUrl(new ImageUrl
                    {
                        Url = "https://example.com/image.jpg",
                        Detail = ImageDetail.High
                    })
                })
            }
        }
    });
    Console.WriteLine(response.Choices[0].Message.Text());
    ```

=== "Ruby"

    ```ruby
    require "liter_llm"

    client = LiterLlm.create_client("sk-...", nil, nil, nil, nil)

    response = client.chat(
      model: "gpt-4o",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What is in this image?" },
            {
              type: "image_url",
              image_url: {
                url: "https://example.com/image.jpg",
                detail: "high"
              }
            }
          ]
        }
      ]
    )

    puts response.choices[0].message.text()
    ```

=== "PHP"

    ```php
    <?php
    use Xberg\LiterLlm\LiterLlm;
    use Xberg\LiterLlm\ContentPart;
    use Xberg\LiterLlm\ImageDetail;

    $client = LiterLlm::createClient("sk-...", null, null, null, null);

    $response = $client->chat([
        "model" => "gpt-4o",
        "messages" => [
            [
                "role" => "user",
                "content" => [
                    ["type" => "text", "text" => "What is in this image?"],
                    [
                        "type" => "image_url",
                        "image_url" => [
                            "url" => "https://example.com/image.jpg",
                            "detail" => "high"
                        ]
                    ]
                ]
            ]
        ]
    ]);

    echo $response->choices[0]->message->text();
    ```

=== "Elixir"

    ```elixir
    defmodule MyApp do
      require LiterLlm

      def analyze_image do
        {:ok, client} = LiterLlm.create_client("sk-...", nil, nil, nil, nil)

        {:ok, response} = LiterLlm.Client.chat(client, %{
          model: "gpt-4o",
          messages: [
            %{
              role: "user",
              content: [
                %{type: "text", text: "What is in this image?"},
                %{
                  type: "image_url",
                  image_url: %{
                    url: "https://example.com/image.jpg",
                    detail: "high"
                  }
                }
              ]
            }
          ]
        })

        response.choices
        |> List.first()
        |> Map.get(:message)
        |> Map.get(:text)
        |> IO.puts()
      end
    end
    ```

### Base64 Data URL

Embed images directly as base64 data URLs without hosting them remotely.

=== "Python"

    ```python
    from liter_llm import create_client, image
    from pathlib import Path

    client = create_client(api_key="sk-...")

    # Encode local file
    png_bytes = Path("photo.png").read_bytes()
    data_url = image.encode_data_url(png_bytes, image.IMAGE_PNG)

    response = client.chat(
        model="gpt-4o",
        messages=[
            {
                "role": "user",
                "content": [
                    {"type": "text", "text": "Describe this image"},
                    {
                        "type": "image_url",
                        "image_url": {"url": data_url}
                    }
                ]
            }
        ]
    )
    print(response.choices[0].message.text())
    ```

=== "Rust"

    ```rust
    use liter_llm::{create_client, image, ContentPart, ImageUrl};
    use std::fs;

    let client = create_client("sk-...", None, None, None, None)?;
    let png_bytes = fs::read("photo.png")?;
    let data_url = image::encode_data_url(&png_bytes, Some(image::IMAGE_PNG));

    let response = client.chat(&liter_llm::ChatCompletionRequest {
        model: "gpt-4o".into(),
        messages: vec![
            liter_llm::Message::User(liter_llm::UserMessage {
                content: liter_llm::UserContent::Parts(vec![
                    ContentPart::Text {
                        text: "Describe this image".into(),
                    },
                    ContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: data_url,
                            detail: None,
                        },
                    },
                ]),
                name: None,
            }),
        ],
        ..Default::default()
    }).await?;

    println!("{:?}", response.choices[0].message.text());
    ```

=== "TypeScript"

    ```typescript
    import { createClient, encodeDataUrl, IMAGE_PNG } from "@xberg-io/liter-llm";
    import { readFileSync } from "fs";

    const client = createClient(process.env.OPENAI_API_KEY!);

    // Encode local file
    const pngBytes = readFileSync("photo.png");
    const dataUrl = encodeDataUrl(pngBytes, IMAGE_PNG);

    const response = await client.chat({
        model: "gpt-4o",
        messages: [
            {
                role: "user",
                content: [
                    { type: "text", text: "Describe this image" },
                    { type: "image_url", image_url: { url: dataUrl } }
                ]
            }
        ]
    });

    console.log(response.choices[0].message.text());
    ```

=== "Go"

    ```go
    package main

    import (
        "fmt"
        "os"
        llm "liter-llm"
    )

    func main() {
        client, _ := llm.CreateClient("sk-...", nil, nil, nil, nil)

        pngBytes, _ := os.ReadFile("photo.png")
        dataUrl := llm.EncodeDataUrl(pngBytes, llm.ImagePng)

        response, _ := client.Chat(&llm.ChatCompletionRequest{
            Model: "gpt-4o",
            Messages: []llm.Message{
                &llm.UserMessage{
                    Content: llm.NewUserContentParts([]llm.ContentPart{
                        llm.NewContentPartText("Describe this image"),
                        llm.NewContentPartImageUrl(llm.ImageUrl{
                            URL: dataUrl,
                        }),
                    }),
                },
            },
        })

        fmt.Println(response.Choices[0].Message.Text())
    }
    ```

=== "Java"

    ```java
    import io.xberg.literllm.*;
    import java.nio.file.Files;
    import java.nio.file.Paths;

    DefaultClient client = LiterLlm.createClient("sk-...", null, null, null, null);

    byte[] pngBytes = Files.readAllBytes(Paths.get("photo.png"));
    String dataUrl = LiterLlm.encodeDataUrl(pngBytes, "image/png");

    ChatCompletionRequest req = new ChatCompletionRequest()
        .model("gpt-4o")
        .messages(Arrays.asList(
            new UserMessage()
                .content(new UserContent(Arrays.asList(
                    ContentPart.text("Describe this image"),
                    ContentPart.imageUrl(new ImageUrl().url(dataUrl))
                )))
        ));

    ChatCompletionResponse resp = client.chat(req);
    System.out.println(resp.getChoices().get(0).getMessage().text());
    ```

=== "C#"

    ```csharp
    using LiterLlm;
    using System.IO;

    var client = LiterLlm.CreateClient("sk-...", null, null, null, null);

    var pngBytes = File.ReadAllBytes("photo.png");
    var dataUrl = LiterLlm.EncodeDataUrl(pngBytes, "image/png");

    var response = await client.ChatAsync(new ChatCompletionRequest
    {
        Model = "gpt-4o",
        Messages = new List<Message>
        {
            new UserMessage
            {
                Content = new UserContent(new List<ContentPart>
                {
                    ContentPart.Text("Describe this image"),
                    ContentPart.ImageUrl(new ImageUrl { Url = dataUrl })
                })
            }
        }
    });

    Console.WriteLine(response.Choices[0].Message.Text());
    ```

=== "Ruby"

    ```ruby
    require "liter_llm"

    client = LiterLlm.create_client("sk-...", nil, nil, nil, nil)

    png_bytes = File.read("photo.png", mode: "rb")
    data_url = LiterLlm.encode_data_url(png_bytes, "image/png")

    response = client.chat(
      model: "gpt-4o",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "Describe this image" },
            { type: "image_url", image_url: { url: data_url } }
          ]
        }
      ]
    )

    puts response.choices[0].message.text()
    ```

=== "PHP"

    ```php
    <?php
    use Xberg\LiterLlm\LiterLlm;

    $client = LiterLlm::createClient("sk-...", null, null, null, null);

    $pngBytes = file_get_contents("photo.png");
    $dataUrl = LiterLlm::encodeDataUrl($pngBytes, "image/png");

    $response = $client->chat([
        "model" => "gpt-4o",
        "messages" => [
            [
                "role" => "user",
                "content" => [
                    ["type" => "text", "text" => "Describe this image"],
                    [
                        "type" => "image_url",
                        "image_url" => ["url" => $dataUrl]
                    ]
                ]
            ]
        ]
    ]);

    echo $response->choices[0]->message->text();
    ```

## Structured JSON Output

Request responses in a specific JSON schema format.

### JSON Object

Simple JSON mode (any valid JSON object).

=== "Python"

    ```python
    from liter_llm import create_client, ResponseFormat

    client = create_client(api_key="sk-...")

    response = client.chat(
        model="gpt-4o",
        messages=[
            {
                "role": "user",
                "content": "Extract the main entities from: 'John Smith works at Acme Corp in NYC'"
            }
        ],
        response_format=ResponseFormat.json_object()
    )
    print(response.choices[0].message.text())
    ```

=== "Rust"

    ```rust
    use liter_llm::{create_client, ResponseFormat};

    let client = create_client("sk-...", None, None, None, None)?;

    let response = client.chat(&liter_llm::ChatCompletionRequest {
        model: "gpt-4o".into(),
        messages: vec![
            liter_llm::Message::User(liter_llm::UserMessage {
                content: "Extract the main entities from: 'John Smith works at Acme Corp in NYC'".into(),
                ..Default::default()
            }),
        ],
        response_format: Some(ResponseFormat::json_object()),
        ..Default::default()
    }).await?;

    println!("{}", response.choices[0].message.text()?);
    ```

=== "TypeScript"

    ```typescript
    import { createClient, ResponseFormat } from "@xberg-io/liter-llm";

    const client = createClient(process.env.OPENAI_API_KEY!);

    const response = await client.chat({
        model: "gpt-4o",
        messages: [
            {
                role: "user",
                content: "Extract the main entities from: 'John Smith works at Acme Corp in NYC'"
            }
        ],
        responseFormat: ResponseFormat.jsonObject()
    });

    console.log(response.choices[0].message.text());
    ```

=== "Go"

    ```go
    package main

    import (
        "fmt"
        llm "liter-llm"
    )

    func main() {
        client, _ := llm.CreateClient("sk-...", nil, nil, nil, nil)

        response, _ := client.Chat(&llm.ChatCompletionRequest{
            Model: "gpt-4o",
            Messages: []llm.Message{
                &llm.UserMessage{
                    Content: "Extract the main entities from: 'John Smith works at Acme Corp in NYC'",
                },
            },
            ResponseFormat: llm.ResponseFormatJsonObject(),
        })

        fmt.Println(response.Choices[0].Message.Text())
    }
    ```

=== "Java"

    ```java
    import io.xberg.literllm.*;

    DefaultClient client = LiterLlm.createClient("sk-...", null, null, null, null);

    ChatCompletionRequest req = new ChatCompletionRequest()
        .model("gpt-4o")
        .messages(Arrays.asList(
            new UserMessage()
                .content("Extract the main entities from: 'John Smith works at Acme Corp in NYC'")
        ))
        .responseFormat(ResponseFormat.jsonObject());

    ChatCompletionResponse resp = client.chat(req);
    System.out.println(resp.getChoices().get(0).getMessage().text());
    ```

=== "C#"

    ```csharp
    using LiterLlm;

    var client = LiterLlm.CreateClient("sk-...", null, null, null, null);

    var response = await client.ChatAsync(new ChatCompletionRequest
    {
        Model = "gpt-4o",
        Messages = new List<Message>
        {
            new UserMessage
            {
                Content = "Extract the main entities from: 'John Smith works at Acme Corp in NYC'"
            }
        },
        ResponseFormat = ResponseFormat.JsonObject()
    });

    Console.WriteLine(response.Choices[0].Message.Text());
    ```

=== "Ruby"

    ```ruby
    require "liter_llm"

    client = LiterLlm.create_client("sk-...", nil, nil, nil, nil)

    response = client.chat(
      model: "gpt-4o",
      messages: [
        {
          role: "user",
          content: "Extract the main entities from: 'John Smith works at Acme Corp in NYC'"
        }
      ],
      response_format: LiterLlm.response_format_json_object
    )

    puts response.choices[0].message.text()
    ```

### JSON Schema (Strict)

Define an exact schema the model must follow.

=== "Python"

    ```python
    from liter_llm import create_client, ResponseFormat

    client = create_client(api_key="sk-...")

    schema = {
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "company": {"type": "string"},
            "location": {"type": "string"}
        },
        "required": ["name", "company", "location"]
    }

    response = client.chat(
        model="gpt-4o",
        messages=[
            {
                "role": "user",
                "content": "Extract the main entities from: 'John Smith works at Acme Corp in NYC'"
            }
        ],
        response_format=ResponseFormat.json_schema(
            name="Entity",
            schema=schema
        )
    )
    print(response.choices[0].message.text())
    ```

=== "Rust"

    ```rust
    use liter_llm::{create_client, ResponseFormat, JsonSchemaFormat};
    use serde_json::json;

    let client = create_client("sk-...", None, None, None, None)?;

    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "company": {"type": "string"},
            "location": {"type": "string"}
        },
        "required": ["name", "company", "location"]
    });

    let response = client.chat(&liter_llm::ChatCompletionRequest {
        model: "gpt-4o".into(),
        messages: vec![
            liter_llm::Message::User(liter_llm::UserMessage {
                content: "Extract the main entities from: 'John Smith works at Acme Corp in NYC'".into(),
                ..Default::default()
            }),
        ],
        response_format: Some(
            ResponseFormat::json_schema("Entity", schema)
        ),
        ..Default::default()
    }).await?;

    println!("{}", response.choices[0].message.text()?);
    ```

## Multimodal Output

### Image Generation

Generate images from text prompts.

=== "Python"

    ```python
    from liter_llm import create_client

    client = create_client(api_key="sk-...")

    response = client.image_generate(
        model="dall-e-3",
        prompt="A serene landscape with mountains and lake"
    )

    for image in response.data:
        print(f"Image URL: {image.url}")
    ```

### Image Output (Gemini)

Request image output directly in chat completion.

=== "Python"

    ```python
    from liter_llm import create_client, Modality

    client = create_client(api_key="sk-...")

    response = client.chat(
        model="gemini-2.0-flash",
        messages=[
            {
                "role": "user",
                "content": "Generate a serene landscape image"
            }
        ],
        modalities=["image"]
    )

    # Access output images
    output_images = response.choices[0].message.output_images()
    for img in output_images:
        print(f"Generated image: {img.url}")
    ```

=== "Rust"

    ```rust
    use liter_llm::{create_client, Modality};

    let client = create_client("sk-...", None, None, None, None)?;

    let response = client.chat(&liter_llm::ChatCompletionRequest {
        model: "gemini-2.0-flash".into(),
        messages: vec![
            liter_llm::Message::User(liter_llm::UserMessage {
                content: "Generate a serene landscape image".into(),
                ..Default::default()
            }),
        ],
        modalities: Some(vec![Modality::Image]),
        ..Default::default()
    }).await?;

    let output_images = response.choices[0].message.output_images();
    for img in output_images {
        println!("Generated image: {}", img.url);
    }
    ```

### Audio Output (OpenAI)

Request audio output from speech models.

=== "Python"

    ```python
    from liter_llm import create_client, Modality
    from pathlib import Path

    client = create_client(api_key="sk-...")

    response = client.chat(
        model="gpt-4o-audio-preview",
        messages=[
            {
                "role": "user",
                "content": "Tell me about the history of AI"
            }
        ],
        modalities=["text", "audio"]
    )

    # Extract text and audio
    text = response.choices[0].message.text()
    audio_parts = response.choices[0].message.output_audio()

    for audio in audio_parts:
        # audio.data is base64, audio.format is the codec
        Path("response.wav").write_bytes(
            __import__("base64").b64decode(audio.data)
        )
    ```

=== "Rust"

    ```rust
    use liter_llm::{create_client, Modality};
    use std::fs;

    let client = create_client("sk-...", None, None, None, None)?;

    let response = client.chat(&liter_llm::ChatCompletionRequest {
        model: "gpt-4o-audio-preview".into(),
        messages: vec![
            liter_llm::Message::User(liter_llm::UserMessage {
                content: "Tell me about the history of AI".into(),
                ..Default::default()
            }),
        ],
        modalities: Some(vec![Modality::Text, Modality::Audio]),
        ..Default::default()
    }).await?;

    let msg = &response.choices[0].message;
    println!("Text: {:?}", msg.text());

    for audio in msg.output_audio() {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&audio.data)?;
        fs::write("response.wav", decoded)?;
    }
    ```

=== "TypeScript"

    ```typescript
    import { createClient, Modality } from "@xberg-io/liter-llm";
    import { writeFileSync } from "fs";

    const client = createClient(process.env.OPENAI_API_KEY!);

    const response = await client.chat({
        model: "gpt-4o-audio-preview",
        messages: [
            {
                role: "user",
                content: "Tell me about the history of AI"
            }
        ],
        modalities: ["text", "audio"]
    });

    const text = response.choices[0].message.text();
    const audioParts = response.choices[0].message.outputAudio();

    console.log("Text:", text);
    for (const audio of audioParts) {
        const decoded = Buffer.from(audio.data, "base64");
        writeFileSync("response.wav", decoded);
    }
    ```

=== "Go"

    ```go
    package main

    import (
        "encoding/base64"
        "fmt"
        "os"
        llm "liter-llm"
    )

    func main() {
        client, _ := llm.CreateClient("sk-...", nil, nil, nil, nil)

        response, _ := client.Chat(&llm.ChatCompletionRequest{
            Model: "gpt-4o-audio-preview",
            Messages: []llm.Message{
                &llm.UserMessage{
                    Content: "Tell me about the history of AI",
                },
            },
            Modalities: []llm.Modality{llm.ModText, llm.ModAudio},
        })

        msg := response.Choices[0].Message
        fmt.Println("Text:", msg.Text())

        for _, audio := range msg.OutputAudio() {
            decoded, _ := base64.StdEncoding.DecodeString(audio.Data)
            os.WriteFile("response.wav", decoded, 0644)
        }
    }
    ```

=== "Java"

    ```java
    import io.xberg.literllm.*;
    import java.nio.file.Files;
    import java.nio.file.Paths;
    import java.util.Base64;

    DefaultClient client = LiterLlm.createClient("sk-...", null, null, null, null);

    ChatCompletionRequest req = new ChatCompletionRequest()
        .model("gpt-4o-audio-preview")
        .messages(Arrays.asList(
            new UserMessage()
                .content("Tell me about the history of AI")
        ))
        .modalities(Arrays.asList(Modality.TEXT, Modality.AUDIO));

    ChatCompletionResponse resp = client.chat(req);
    String text = resp.getChoices().get(0).getMessage().text();
    System.out.println("Text: " + text);

    for (AudioContent audio : resp.getChoices().get(0).getMessage().outputAudio()) {
        byte[] decoded = Base64.getDecoder().decode(audio.data);
        Files.write(Paths.get("response.wav"), decoded);
    }
    ```

=== "C#"

    ```csharp
    using LiterLlm;
    using System.IO;

    var client = LiterLlm.CreateClient("sk-...", null, null, null, null);

    var response = await client.ChatAsync(new ChatCompletionRequest
    {
        Model = "gpt-4o-audio-preview",
        Messages = new List<Message>
        {
            new UserMessage
            {
                Content = "Tell me about the history of AI"
            }
        },
        Modalities = new List<Modality> { Modality.Text, Modality.Audio }
    });

    var text = response.Choices[0].Message.Text();
    Console.WriteLine($"Text: {text}");

    foreach (var audio in response.Choices[0].Message.OutputAudio())
    {
        var decoded = Convert.FromBase64String(audio.Data);
        await File.WriteAllBytesAsync("response.wav", decoded);
    }
    ```

=== "Ruby"

    ```ruby
    require "liter_llm"
    require "base64"

    client = LiterLlm.create_client("sk-...", nil, nil, nil, nil)

    response = client.chat(
      model: "gpt-4o-audio-preview",
      messages: [
        {
          role: "user",
          content: "Tell me about the history of AI"
        }
      ],
      modalities: ["text", "audio"]
    )

    text = response.choices[0].message.text
    puts "Text: #{text}"

    response.choices[0].message.output_audio.each do |audio|
      decoded = Base64.decode64(audio.data)
      File.write("response.wav", decoded)
    end
    ```

## Provider Mapping

Not all providers support all modalities. Refer to the table below for support:

| Feature | OpenAI | Anthropic | Gemini/Vertex | Claude |
|---------|--------|-----------|---------------|--------|
| **Vision Input** | `gpt-4o`, `gpt-4-turbo` | `claude-3.5-sonnet` | `gemini-2.0-flash` | Yes |
| **Response Format (JSON Schema)** | Yes | Yes (via system) | Yes (native) | Via system |
| **Image Output** | `dall-e-3` | — | `imagen-3` (Vertex) | — |
| **Audio Output** | `gpt-4o-audio-*` | — | — | — |

For details on provider-specific transformations, see [Providers](../providers.md).
