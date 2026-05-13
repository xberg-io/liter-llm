```python
import asyncio
import os

from liter_llm import create_client
from liter_llm._internal_bindings import ChatCompletionRequest


async def main() -> None:
    client = create_client(api_key=os.environ["OPENAI_API_KEY"])
    request = ChatCompletionRequest.from_json(
        '{"model":"openai/gpt-4o","messages":[{"role":"user","content":"Explain quantum computing briefly"}],"stream":true}'
    )
    full_text = ""
    async for chunk in client.chat_stream(request):
        delta = chunk.choices[0].delta.content if chunk.choices else None
        if delta:
            full_text += delta
            print(delta, end="", flush=True)
    print()
    print(f"Full response length: {len(full_text)} characters")


asyncio.run(main())
```
