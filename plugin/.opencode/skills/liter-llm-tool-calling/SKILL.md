---
description: Use when defining functions/tools for an LLM to call through liter-llm, or requesting structured JSON outputs. Covers tool schemas, tool_calls handling, and response formats.
name: liter-llm-tool-calling
---

<!--
AI-RULEZ :: GENERATED FILE — DO NOT EDIT
Content-Hash: blake3:f26049693a8fa3229f675eefc4097cfa0aeb669a5744967a7ab3614009a80310
Source-Hash: blake3:d3660ff50caaec152c20dce3dd60288dc9d9290a8e656c7f6d8bb74c7727852d
Schema-Version: v1
-->

# Tool Calling

Pass a `tools` array of function definitions; the model may respond with
`tool_calls` instead of (or alongside) text. Execute the named function and feed
the result back as a `tool` message.

## Python

```python
import asyncio, json, os
from liter_llm import create_client
from liter_llm._internal_bindings import ChatCompletionRequest

payload = {
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
                    "properties": {
                        "location": {"type": "string", "description": "City name"},
                    },
                    "required": ["location"],
                },
            },
        }
    ],
    "tool_choice": "auto",
}

async def main() -> None:
    client = create_client(api_key=os.environ["OPENAI_API_KEY"])
    request = ChatCompletionRequest.from_json(json.dumps(payload))
    response = await client.chat(request)
    for call in response.choices[0].message.tool_calls or []:
        print(call.function.name, call.function.arguments)  # arguments is a JSON string

asyncio.run(main())
```

## Structured outputs

Request strict JSON with `response_format`:

```python
request = ChatCompletionRequest.from_json(json.dumps({
    "model": "openai/gpt-4o",
    "messages": [{"role": "user", "content": "Extract name and age as JSON."}],
    "response_format": {"type": "json_object"},
}))
response = await client.chat(request)
```

## Notes

- `function.arguments` is a JSON **string** — parse it before use.
- Append each tool result as a message with `role="tool"` and the matching
  `tool_call_id`, then call `chat` again to let the model continue.
- Tool support and JSON-mode availability vary by provider; check the provider
  reference if a model ignores `tools`.
