<!-- snippet:compile-only -->

```python
import asyncio
import os
from pathlib import Path

from liter_llm import create_client
from liter_llm._internal_bindings import CreateSpeechRequest


async def main() -> None:
    client = create_client(api_key=os.environ["OPENAI_API_KEY"])
    request = CreateSpeechRequest.from_json(
        '{"model":"openai/tts-1","input":"Hello, world!","voice":"alloy"}'
    )
    audio_bytes = await client.speech(request)
    Path("output.mp3").write_bytes(audio_bytes)
    print(f"Wrote {len(audio_bytes)} bytes to output.mp3")


asyncio.run(main())
```
