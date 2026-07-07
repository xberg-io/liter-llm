<!-- snippet:compile-only -->

```python
import asyncio
import os

from liter_llm import create_client
from liter_llm._internal_bindings import CreateImageRequest


async def main() -> None:
    client = create_client(api_key=os.environ["OPENAI_API_KEY"])
    request = CreateImageRequest.from_json(
        '{"model":"openai/dall-e-3","prompt":"A sunset over mountains","n":1,"size":"1024x1024"}'
    )
    response = await client.image_generate(request)
    print(response.data[0].url)


asyncio.run(main())
```
