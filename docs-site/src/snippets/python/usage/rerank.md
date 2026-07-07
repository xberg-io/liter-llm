<!-- snippet:compile-only -->

```python
import asyncio
import json
import os

from liter_llm import create_client
from liter_llm._internal_bindings import RerankRequest


async def main() -> None:
    client = create_client(api_key=os.environ["COHERE_API_KEY"])
    payload = {
        "model": "cohere/rerank-v3.5",
        "query": "What is the capital of France?",
        "documents": [
            "Paris is the capital of France.",
            "Berlin is the capital of Germany.",
            "London is the capital of England.",
        ],
    }
    request = RerankRequest.from_json(json.dumps(payload))
    response = await client.rerank(request)
    for result in response.results:
        print(f"Index: {result.index}, Score: {result.relevance_score:.4f}")


asyncio.run(main())
```
