<!-- snippet:compile-only -->

```python
import asyncio
import os

from liter_llm import create_client
from liter_llm._internal_bindings import SearchRequest


async def main() -> None:
    client = create_client(api_key=os.environ["BRAVE_API_KEY"])
    request = SearchRequest.from_json(
        '{"model":"brave/web-search","query":"What is Rust programming language?","max_results":5}'
    )
    response = await client.search(request)
    for result in response.results:
        print(f"{result.title}: {result.url}")


asyncio.run(main())
```
