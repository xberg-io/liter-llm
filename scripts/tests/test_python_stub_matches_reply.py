"""Verify the Python type stub describes the chat reply a caller gets.

Task `python:e2e:test` runs it with the generated Python e2e suite, after the package is built. To run it alone:
    cd e2e/python && uv run --no-sync pytest ../../scripts/tests/test_python_stub_matches_reply.py -q
"""

import ast
import json
import pathlib
import unittest

import liter_llm

STUB = pathlib.Path(liter_llm.__file__).parent / "_internal_bindings.pyi"
REPLY = {
    "id": "c1",
    "object": "chat.completion",
    "created": 1,
    "model": "m",
    "choices": [{"index": 0, "finish_reason": "stop", "message": {"role": "assistant", "content": "hello"}}],
}


def _declared(tree: ast.Module, name: str) -> set[str]:
    """Return the methods and attributes the stub declares on class *name*, dunders left out."""
    for node in tree.body:
        if isinstance(node, ast.ClassDef) and node.name == name:
            names: set[str] = set()
            for item in node.body:
                if isinstance(item, ast.FunctionDef) and not item.name.startswith("__"):
                    names.add(item.name)
                elif isinstance(item, ast.AnnAssign) and isinstance(item.target, ast.Name):
                    names.add(item.target.id)
            return names
    raise AssertionError(f"no class {name} in {STUB}")


def _runtime(obj: object) -> set[str]:
    """Return the public names the runtime object has."""
    return {n for n in dir(obj) if not n.startswith("_")}


class StubMatchesReplyTests(unittest.TestCase):
    """The stub for a parsed reply and the parsed reply must declare the same public names."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.tree = ast.parse(STUB.read_text())
        message = liter_llm.ChatCompletionResponse.from_json(json.dumps(REPLY)).choices[0].message
        cls.objects = {"AssistantMessage": message, "AssistantContent": message.content}

    def test_message_stub_declares_no_method_the_reply_lacks(self) -> None:
        """Every name the stub declares on the message exists on the parsed message."""
        stub, real = _declared(self.tree, "AssistantMessage"), _runtime(self.objects["AssistantMessage"])
        assert sorted(stub - real) == []

    def test_content_stub_declares_text_and_parts(self) -> None:
        """The stub declares the text and parts accessors the parsed content has."""
        stub, real = _declared(self.tree, "AssistantContent"), _runtime(self.objects["AssistantContent"])
        assert sorted(real - stub) == []
        assert {"text", "parts"} <= stub


if __name__ == "__main__":
    unittest.main()
