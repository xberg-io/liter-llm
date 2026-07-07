<!-- snippet:compile-only -->

```python
import asyncio
import os

from liter_llm import create_client
from liter_llm._internal_bindings import OcrRequest


async def main() -> None:
    client = create_client(api_key=os.environ["MISTRAL_API_KEY"])
    request = OcrRequest.from_json(
        '{"model":"mistral/mistral-ocr-latest",'
        '"document":{"type":"document_url","url":"https://example.com/invoice.pdf"}}'
    )
    response = await client.ocr(request)
    for page in response.pages:
        print(f"Page {page.index}: {page.markdown[:100]}...")


asyncio.run(main())
```
