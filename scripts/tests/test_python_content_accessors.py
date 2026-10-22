"""Verify the Python reply content and embedding input expose their values through the documented getters.

The test builds a real chat call against a local stub server. Task `python:e2e:test` runs it with the
generated Python e2e suite, after the package is built.
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
        """A reply whose content is a list of parts has .parts with both part values and .text of None."""
        content = self._content(PARTS)
        assert content.text is None
        assert content.parts is not None
        assert [part["text"] for part in content.parts] == ["hello", " world"]
        assert "".join(part["text"] for part in content.parts) == "hello world"

    def test_empty_text_reply_gives_empty_text(self) -> None:
        """An empty string is a text reply: .text is the empty string, not None."""
        content = self._content("")
        assert content.text == ""
        assert content.parts is None

    def test_empty_parts_reply_gives_empty_parts(self) -> None:
        """An empty list is a parts reply: .parts is the empty list, not None."""
        content = self._content([])
        assert content.text is None
        assert content.parts == []


class EmbeddingInputAccessorTests(unittest.TestCase):
    """An embedding input must give back the string or the list of strings it holds."""

    def test_single_input_gives_single_and_no_multiple(self) -> None:
        """An input built from one string has .single equal to the string and .multiple of None."""
        value = liter_llm.EmbeddingInput.from_single("alpha")
        assert value.single == "alpha"
        assert value.multiple is None

    def test_multiple_input_gives_multiple_and_no_single(self) -> None:
        """An input built from a list of strings has .multiple equal to the list and .single of None."""
        value = liter_llm.EmbeddingInput.from_multiple(["alpha", "beta"])
        assert value.multiple == ["alpha", "beta"]
        assert value.single is None

    def test_empty_inputs_keep_their_kind(self) -> None:
        """An empty string stays a single input and an empty list stays a multiple input."""
        assert liter_llm.EmbeddingInput.from_single("").single == ""
        assert liter_llm.EmbeddingInput.from_multiple([]).multiple == []

    def test_constructor_reads_the_value_by_its_shape(self) -> None:
        """The constructor reads a plain string or list by its JSON shape, so the matching getter returns it."""
        assert liter_llm.EmbeddingInput("alpha").single == "alpha"
        assert liter_llm.EmbeddingInput(["alpha", "beta"]).multiple == ["alpha", "beta"]


if __name__ == "__main__":
    unittest.main()
