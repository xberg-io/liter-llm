---
description: Use when sending chat completions through liter-llm and routing to a specific provider via the `provider/model` prefix. Covers the chat call shape, provider routing, model_hint, message roles, and error categories.
name: liter-llm-calling-llms
---

<!--
AI-RULEZ :: GENERATED FILE — DO NOT EDIT
Content-Hash: blake3:d45c8ad7f731f0e0bc91646e677fd7c89b6af31e38944bff4302e7e242a12906
Source-Hash: blake3:88c87867f5079f6c8fcb582aee913cca4f6feefe541c5361d2307a9b45233f97
Schema-Version: v1
-->

# Calling LLMs

Build a `ChatCompletionRequest` and send it with `client.chat(request)`. Create
the client with `create_client(...)`. The model string is `provider/model`; the
prefix selects the backend.

```python
import asyncio, json, os
from liter_llm import create_client
from liter_llm._internal_bindings import ChatCompletionRequest

async def main() -> None:
    client = create_client(api_key=os.environ["OPENAI_API_KEY"])
    request = ChatCompletionRequest.from_json(json.dumps({
        "model": "openai/gpt-4o",
        "messages": [
            {"role": "system", "content": "You are concise."},
            {"role": "user", "content": "Name three Rust crates for HTTP."},
        ],
    }))
    response = await client.chat(request)
    print(response.choices[0].message.content)

asyncio.run(main())
```

## Provider routing

The model string's prefix selects the provider; build a request per backend:

```python
ChatCompletionRequest.from_json('{"model":"anthropic/claude-sonnet-4-20250514","messages":[...]}')
ChatCompletionRequest.from_json('{"model":"gemini/gemini-2.0-flash","messages":[...]}')
ChatCompletionRequest.from_json('{"model":"groq/llama3-70b","messages":[...]}')
ChatCompletionRequest.from_json('{"model":"mistral/mistral-large-latest","messages":[...]}')
ChatCompletionRequest.from_json('{"model":"bedrock/anthropic.claude-v2","messages":[...]}')
```

Set a full `provider/model` as `model_hint` at construction to select the client
provider and credential source. Requests without a recognized prefix fall back to
that client provider:

```python
client = create_client(api_key=os.environ["OPENAI_API_KEY"], model_hint="openai/gpt-4o")
# the request model can now omit the provider prefix:
request = ChatCompletionRequest.from_json('{"model":"gpt-4o","messages":[...]}')
await client.chat(request)  # routes to OpenAI
```

## Notes

- Keys come from env vars (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, …); never
  hardcode them.
- Without a recognized prefix, requests fall back to the client's configured provider;
  a client without `model_hint` defaults to OpenAI.
- Environment credentials are selected when constructing the client, not independently
  for every request. Use a separate client with matching `model_hint` and credentials
  for each provider. A prefix changes routing but does not switch the client's API key.
- Python errors are typed exceptions exported from `liter_llm`:
  `AuthenticationError`, `RateLimitedError`, `BadRequestError`,
  `ContextWindowExceededError`, `ContentPolicyError`, `NotFoundError`,
  `ServerError`, `ServiceUnavailableError`, `LiterLlmTimeoutError`,
  `BudgetExceededError` — all subclasses of `LiterLlmError`.
