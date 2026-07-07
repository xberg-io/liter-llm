```python
import asyncio
import os

from liter_llm import (
    AuthenticationError,
    BudgetExceededError,
    ContextWindowExceededError,
    LiterLlmError,
    RateLimitedError,
    create_client,
)
from liter_llm._internal_bindings import ChatCompletionRequest


async def main() -> None:
    client = create_client(api_key=os.environ["OPENAI_API_KEY"])
    request = ChatCompletionRequest.from_json(
        '{"model":"openai/gpt-4o","messages":[{"role":"user","content":"Hello"}]}'
    )
    try:
        response = await client.chat(request)
        print(response.choices[0].message.content)
    except AuthenticationError as e:
        # 401/403 -- rotate the key, do not retry.
        print(f"auth failed: {e}")
    except RateLimitedError as e:
        # 429 -- transient, retry with backoff or fall back to another model.
        print(f"rate limited: {e}")
    except ContextWindowExceededError as e:
        # Trim the prompt or use a larger context window.
        print(f"prompt too long: {e}")
    except BudgetExceededError as e:
        # Virtual-key or global budget cap hit.
        print(f"budget exceeded: {e}")
    except LiterLlmError as e:
        # Catch-all for the remaining liter-llm errors.
        print(f"llm error: {e}")


asyncio.run(main())
```
