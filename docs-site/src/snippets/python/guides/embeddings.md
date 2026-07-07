```python
import asyncio
import os

from liter_llm import create_client
from liter_llm._internal_bindings import EmbeddingRequest


async def main() -> None:
    client = create_client(api_key=os.environ["OPENAI_API_KEY"])
    request = EmbeddingRequest.from_json(
        '{"model":"openai/text-embedding-3-small","input":["The quick brown fox jumps over the lazy dog"]}'
    )
    response = await client.embed(request)
    print(f"Dimensions: {len(response.data[0].embedding)}")
    print(f"First 5 values: {response.data[0].embedding[:5]}")


asyncio.run(main())
```
