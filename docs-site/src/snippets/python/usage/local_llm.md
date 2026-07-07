```python
import asyncio

from liter_llm import create_client
from liter_llm._internal_bindings import ChatCompletionRequest


async def main() -> None:
    # No API key needed for local providers
    client = create_client(api_key="", base_url="http://localhost:11434/v1")
    request = ChatCompletionRequest.from_json(
        '{"model":"ollama/qwen2:0.5b","messages":[{"role":"user","content":"Hello!"}]}'
    )
    response = await client.chat(request)
    print(response.choices[0].message.content)


asyncio.run(main())
```
