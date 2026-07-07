```python
import asyncio
import os

from liter_llm import create_client
from liter_llm._internal_bindings import ChatCompletionRequest


async def main() -> None:
    client = create_client(api_key=os.environ["OPENAI_API_KEY"])
    request = ChatCompletionRequest.from_json(
        '{"model":"openai/gpt-4o","messages":[{"role":"user","content":"Tell me a story"}],"stream":true}'
    )
    async for chunk in client.chat_stream(request):
        if chunk.choices and chunk.choices[0].delta.content:
            print(chunk.choices[0].delta.content, end="", flush=True)
    print()


asyncio.run(main())
```
