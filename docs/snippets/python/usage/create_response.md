<!-- snippet:compile-only -->

```python
import asyncio
import os

from liter_llm import create_client
from liter_llm._internal_bindings import CreateResponseRequest


async def main() -> None:
    client = create_client(api_key=os.environ["OPENAI_API_KEY"])
    request = CreateResponseRequest.from_json(
        '{"model":"openai/gpt-4o","input":"Explain quantum computing in one sentence."}'
    )
    response = await client.create_response(request)
    print(f"Status: {response.status}")
    for item in response.output:
        print(item.content)


asyncio.run(main())
```
