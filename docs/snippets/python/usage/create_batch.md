<!-- snippet:compile-only -->

```python
import asyncio
import os

from liter_llm import create_client
from liter_llm._internal_bindings import CreateBatchRequest


async def main() -> None:
    client = create_client(api_key=os.environ["OPENAI_API_KEY"])
    request = CreateBatchRequest.from_json(
        '{"input_file_id":"file-abc123","endpoint":"/v1/chat/completions","completion_window":"24h"}'
    )
    response = await client.create_batch(request)
    print(f"Batch ID: {response.id}")
    print(f"Status: {response.status}")


asyncio.run(main())
```
