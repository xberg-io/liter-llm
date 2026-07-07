<!-- snippet:compile-only -->

```python
import asyncio
import os

from liter_llm import create_client
from liter_llm._internal_bindings import ModerationRequest

CATEGORIES = (
    "sexual", "hate", "harassment", "self_harm", "sexual_minors",
    "hate_threatening", "violence_graphic", "self_harm_intent",
    "self_harm_instructions", "harassment_threatening", "violence",
)


async def main() -> None:
    client = create_client(api_key=os.environ["OPENAI_API_KEY"])
    request = ModerationRequest.from_json(
        '{"model":"openai/omni-moderation-latest","input":"This is a test message."}'
    )
    response = await client.moderate(request)
    result = response.results[0]
    print(f"Flagged: {result.flagged}")
    for name in CATEGORIES:
        if getattr(result.categories, name):
            score = getattr(result.category_scores, name)
            print(f"  {name}: {score:.4f}")


asyncio.run(main())
```
