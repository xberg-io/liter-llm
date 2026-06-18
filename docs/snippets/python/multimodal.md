# Multimodal I/O in Python

Process images, documents, and audio alongside text using liter-llm's multimodal message API.

## Text with Images

Combine text and image references in a single user message:

```python
import asyncio
import liter_llm
from liter_llm import ChatCompletionRequest, UserMessage, ImageUrl

async def describe_image():
    client = liter_llm.create_client(api_key="your-api-key")
    
    req = ChatCompletionRequest(
        model="gpt-4o-mini",  # Vision-capable model
        messages=[
            UserMessage(content=[
                "What's the main subject in this image?",
                ImageUrl(url="https://example.com/photo.jpg", detail="high")
            ])
        ]
    )
    
    resp = await liter_llm.chat(client, req)
    print(resp.choices[0].message.text())

asyncio.run(describe_image())
```

## Image Data URLs

Embed image bytes inline using base64 data URLs:

```python
from pathlib import Path
import liter_llm
from liter_llm import ChatCompletionRequest, UserMessage, ImageUrl

async def analyze_local_image():
    client = liter_llm.create_client(api_key="your-api-key")
    
    # Read image file
    image_bytes = Path("diagram.png").read_bytes()
    
    # Encode as data URL
    data_url = liter_llm.encode_data_url(image_bytes, mime="image/png")
    
    # Use in message
    req = ChatCompletionRequest(
        model="gpt-4o-mini",
        messages=[
            UserMessage(content=[
                "Analyze this diagram:",
                ImageUrl(url=data_url, detail="high")
            ])
        ]
    )
    
    resp = await liter_llm.chat(client, req)
    print(resp.choices[0].message.text())

asyncio.run(analyze_local_image())
```

## Structured Output (JSON Schema)

Enforce JSON schema conformance for model outputs:

```python
import json
import asyncio
import liter_llm
from liter_llm import (
    ChatCompletionRequest, 
    UserMessage, 
    JsonSchemaFormat
)

async def extract_structured_data():
    client = liter_llm.create_client(api_key="your-api-key")
    
    # Define output schema
    schema = {
        "type": "object",
        "properties": {
            "person_name": {"type": "string"},
            "company": {"type": "string"},
            "role": {"type": "string"}
        },
        "required": ["person_name", "company", "role"]
    }
    
    req = ChatCompletionRequest(
        model="gpt-4o-mini",
        messages=[
            UserMessage(content="Extract from: John Doe works at TechCorp as Senior Engineer")
        ],
        response_format=JsonSchemaFormat(
            name="person_info",
            schema=json.dumps(schema),
            strict=True,
            description="Extract person, company, and role"
        )
    )
    
    resp = await liter_llm.chat(client, req)
    result_text = resp.choices[0].message.text()
    
    # Parse structured output
    extracted = json.loads(result_text)
    print(f"Name: {extracted['person_name']}")
    print(f"Company: {extracted['company']}")
    print(f"Role: {extracted['role']}")

asyncio.run(extract_structured_data())
```

## Multimodal Output

Request multimodal responses (text, images, audio) from capable models:

```python
import asyncio
import liter_llm
from liter_llm import ChatCompletionRequest, UserMessage, Modality

async def multimodal_response():
    client = liter_llm.create_client(api_key="your-api-key")
    
    req = ChatCompletionRequest(
        model="gpt-4o-mini",
        messages=[
            UserMessage(content="Write a short poem and generate accompanying audio")
        ],
        modalities=[Modality.TEXT, Modality.AUDIO]  # Request text + audio
    )
    
    resp = await liter_llm.chat(client, req)
    
    # Extract output
    msg = resp.choices[0].message
    text_output = msg.text()
    audio_outputs = msg.output_audio()
    
    print(f"Text: {text_output}")
    print(f"Audio parts: {len(audio_outputs)}")

asyncio.run(multimodal_response())
```

## Documents and PDFs

Send documents to vision-capable models:

```python
import asyncio
import liter_llm
from liter_llm import (
    ChatCompletionRequest,
    UserMessage,
    DocumentContent
)

async def analyze_document():
    client = liter_llm.create_client(api_key="your-api-key")
    
    # Read PDF and encode
    pdf_bytes = open("report.pdf", "rb").read()
    import base64
    b64_pdf = base64.b64encode(pdf_bytes).decode()
    
    req = ChatCompletionRequest(
        model="gpt-4o-mini",
        messages=[
            UserMessage(content=[
                "Summarize the key findings:",
                DocumentContent(data=b64_pdf, media_type="application/pdf")
            ])
        ]
    )
    
    resp = await liter_llm.chat(client, req)
    print(resp.choices[0].message.text())

asyncio.run(analyze_document())
```

## Audio Input

Transcribe or process audio with speech-capable models:

```python
import asyncio
import liter_llm
from liter_llm import (
    ChatCompletionRequest,
    UserMessage,
    AudioContent
)

async def process_audio():
    client = liter_llm.create_client(api_key="your-api-key")
    
    # Read audio and encode
    audio_bytes = open("recording.mp3", "rb").read()
    import base64
    b64_audio = base64.b64encode(audio_bytes).decode()
    
    req = ChatCompletionRequest(
        model="gpt-4o-mini",
        messages=[
            UserMessage(content=[
                "Transcribe and summarize:",
                AudioContent(data=b64_audio, format="mp3")
            ])
        ]
    )
    
    resp = await liter_llm.chat(client, req)
    print(resp.choices[0].message.text())

asyncio.run(process_audio())
```

## Decoding Data URLs

Extract bytes from data URLs:

```python
import liter_llm

# Encode
encoded = liter_llm.encode_data_url(b"image data", mime="image/jpeg")

# Decode
decoded = liter_llm.decode_data_url(encoded)
if decoded:
    print(f"MIME: {decoded.mime}")
    print(f"Size: {len(decoded.data)} bytes")
```
