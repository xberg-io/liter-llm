```python
import asyncio
import os

from liter_llm import create_client
from liter_llm._internal_bindings import ChatCompletionRequest


async def main() -> None:
    client = create_client(
        api_key=os.environ["OPENAI_API_KEY"],
        base_url=None,             # override provider base URL
        model_hint="openai",       # pre-resolve provider at construction
        max_retries=3,             # retry on transient failures
        timeout_secs=60,           # request timeout in seconds
    )
    request = ChatCompletionRequest.from_json(
        '{"model":"openai/gpt-4o","messages":[{"role":"user","content":"Hello!"}]}'
    )
    response = await client.chat(request)
    print(response.choices[0].message.content)


asyncio.run(main())
```
