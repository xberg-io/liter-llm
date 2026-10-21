"""Verify the Python reply content exposes its text and parts through a real chat call.

Run with the liter-llm package installed, for example:
    uv run --project e2e/python python -m pytest -q scripts/tests/test_python_content_accessors.py
"""

import asyncio
import json
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import liter_llm

PLAIN = "hello"
PARTS = [{"type": "text", "text": "hello"}, {"type": "text", "text": " world"}]


def _reply(content: object) -> bytes:
    return json.dumps(
        {
            "id": "c1",
            "object": "chat.completion",
            "created": 1,
            "model": "m",
            "choices": [{"index": 0, "finish_reason": "stop", "message": {"role": "assistant", "content": content}}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
        }
    ).encode()


class _Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    content: object = PLAIN

    def log_message(self, *args: object) -> None:
        pass

    def do_POST(self) -> None:  # noqa: N802
        self.rfile.read(int(self.headers.get("Content-Length") or 0))
        body = _reply(type(self).content)
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


class ContentAccessorTests(unittest.TestCase):
    """A reply returned by a chat call must give its text and parts to Python code."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.server = ThreadingHTTPServer(("127.0.0.1", 0), _Handler)
        threading.Thread(target=cls.server.serve_forever, daemon=True).start()
        base = f"http://127.0.0.1:{cls.server.server_address[1]}/v1"
        cls.client = liter_llm.create_client_with_options(
            liter_llm.ClientOptions(api_key="k", base_url=base, model_hint="openai", max_retries=0)
        )

    @classmethod
    def tearDownClass(cls) -> None:
        cls.server.shutdown()
        cls.server.server_close()

    def _content(self, wire_content: object) -> object:
        _Handler.content = wire_content
        request = {"model": "m", "messages": [{"role": "user", "content": "hi"}]}
        reply = asyncio.run(liter_llm.chat(self.client, request))
        return reply.choices[0].message.content

    def test_plain_reply_gives_text_and_no_parts(self) -> None:
        """A reply whose content is a plain string has .text equal to the string and .parts of None."""
        content = self._content(PLAIN)
        assert content.text == "hello"
        assert content.parts is None

    def test_parts_reply_gives_parts_and_no_text(self) -> None:
        """A reply whose content is a list of parts has .parts with both parts and .text of None."""
        content = self._content(PARTS)
        assert content.text is None
        assert content.parts is not None
        assert len(content.parts) == 2


if __name__ == "__main__":
    unittest.main()
