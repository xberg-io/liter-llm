```python
import asyncio
import json
import os

from liter_llm import create_client
from liter_llm._internal_bindings import ChatCompletionRequest

REQUEST = {
    "model": "openai/gpt-4o",
    "messages": [{"role": "user", "content": "What is the weather in Berlin?"}],
    "tools": [
        {
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get the current weather for a location",
                "parameters": {
                    "type": "object",
                    "properties": {"location": {"type": "string"}},
                    "required": ["location"],
                },
            },
        }
    ],
    "tool_choice": "auto",
}


async def main() -> None:
    client = create_client(api_key=os.environ["OPENAI_API_KEY"])
    request = ChatCompletionRequest.from_json(json.dumps(REQUEST))
    response = await client.chat(request)
    for call in response.choices[0].message.tool_calls or []:
        print(f"Tool: {call.function.name}, Args: {call.function.arguments}")


asyncio.run(main())
```
