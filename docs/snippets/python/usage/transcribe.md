<!-- snippet:compile-only -->

```python
import asyncio
import base64
import json
import os
from pathlib import Path

from liter_llm import create_client
from liter_llm._internal_bindings import CreateTranscriptionRequest


async def main() -> None:
    client = create_client(api_key=os.environ["OPENAI_API_KEY"])
    encoded = base64.b64encode(Path("audio.mp3").read_bytes()).decode("ascii")
    request = CreateTranscriptionRequest.from_json(
        json.dumps({"model": "openai/whisper-1", "file": encoded})
    )
    response = await client.transcribe(request)
    print(response.text)


asyncio.run(main())
```
