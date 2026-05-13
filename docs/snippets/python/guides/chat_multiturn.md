```python
import asyncio
import json
import os

from liter_llm import create_client
from liter_llm._internal_bindings import ChatCompletionRequest


async def main() -> None:
    client = create_client(api_key=os.environ["OPENAI_API_KEY"])
    messages = [
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "What is the capital of France?"},
    ]

    first = await client.chat(
        ChatCompletionRequest.from_json(json.dumps({"model": "openai/gpt-4o", "messages": messages}))
    )
    reply = first.choices[0].message.content
    print(f"Assistant: {reply}")

    messages.append({"role": "assistant", "content": reply})
    messages.append({"role": "user", "content": "What about Germany?"})

    second = await client.chat(
        ChatCompletionRequest.from_json(json.dumps({"model": "openai/gpt-4o", "messages": messages}))
    )
    print(f"Assistant: {second.choices[0].message.content}")
    if second.usage:
        print(f"Tokens: {second.usage.prompt_tokens} in, {second.usage.completion_tokens} out")


asyncio.run(main())
```
