"""Verify the Python binding passes `ClientOptions.blocked_networks` to the client.

`alef verify` checks that generated files are current, not that the binding glue carries a field. A glue that
drops the list leaves every Rust test green, so this test sets the list from Python and checks the client's
behaviour. Task `python:e2e:test` runs it with the generated Python e2e suite, after the package is built.
To run it alone:
    cd e2e/python && uv run --no-sync pytest ../../scripts/tests/test_python_blocked_networks.py -q
"""

import asyncio
import unittest

import liter_llm
import pytest

# Port 1 refuses the connection at once, so a request that the rule lets through fails with a network error.
BASE_URL = "http://127.0.0.1:1/v1"
REQUEST = {"model": "m", "messages": [{"role": "user", "content": "hi"}]}


def _client(blocked_networks: list[str] | None) -> object:
    options = liter_llm.ClientOptions(
        api_key="k", base_url=BASE_URL, model_hint="openai", max_retries=0, blocked_networks=blocked_networks
    )
    return liter_llm.create_client_with_options(options)


def _chat(blocked_networks: list[str] | None) -> BaseException:
    """Send one chat call and return the error it raises; fail the test when it raises none."""
    client = _client(blocked_networks)
    try:
        asyncio.run(liter_llm.chat(client, REQUEST))
    except liter_llm.LiterLlmError as error:
        return error
    raise AssertionError("the chat call to a closed port returned a reply")


class BlockedNetworksTests(unittest.TestCase):
    """A list set on the options must reach the client that the binding creates."""

    def test_a_listed_network_refuses_the_request(self) -> None:
        """A request to an address in a listed network raises OutboundForbiddenError that names the address."""
        error = _chat(["127.0.0.0/8"])
        assert isinstance(error, liter_llm.OutboundForbiddenError)
        assert "127.0.0.1" in str(error)

    def test_an_unlisted_network_lets_the_request_through(self) -> None:
        """A list that does not hold the address leaves the request to fail on the closed port, not on the rule."""
        assert not isinstance(_chat(["10.0.0.0/8"]), liter_llm.OutboundForbiddenError)

    def test_no_list_and_an_empty_list_set_no_rule(self) -> None:
        """None and an empty list set no rule, so the request fails on the closed port."""
        assert not isinstance(_chat(None), liter_llm.OutboundForbiddenError)
        assert not isinstance(_chat([]), liter_llm.OutboundForbiddenError)

    def test_a_bad_entry_is_refused_when_the_client_is_created(self) -> None:
        """An entry that is not a CIDR network raises BadRequestError that names the entry."""
        for entry in ("not-a-network", "10.0.0.0/99"):
            with self.subTest(entry=entry):
                with pytest.raises(liter_llm.BadRequestError) as caught:
                    _client([entry])
                assert entry in str(caught.value)


if __name__ == "__main__":
    unittest.main()
