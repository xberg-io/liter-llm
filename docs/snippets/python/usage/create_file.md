<!-- snippet:compile-only -->

```python
import asyncio
import base64
import json
import os
from pathlib import Path

from liter_llm import create_client
from liter_llm._internal_bindings import CreateFileRequest


async def main() -> None:
    client = create_client(api_key=os.environ["OPENAI_API_KEY"])
    encoded = base64.b64encode(Path("data.jsonl").read_bytes()).decode("ascii")
    payload = {"file": encoded, "filename": "data.jsonl", "purpose": "batch"}
    request = CreateFileRequest.from_json(json.dumps(payload))
    response = await client.create_file(request)
    print(f"File ID: {response.id}")
    print(f"Size: {response.bytes} bytes")


asyncio.run(main())
```
