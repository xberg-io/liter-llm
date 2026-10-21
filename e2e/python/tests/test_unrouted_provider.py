"""A model for a provider with no route is refused before a request is built.

The spy is a local HTTP proxy. The client reads HTTPS_PROXY and HTTP_PROXY when it is
created, so a request that would leave the process lands on the spy and is recorded.
"""

from __future__ import annotations

import asyncio
import json
import socket
import threading
from collections.abc import Iterator

import pytest
from liter_llm import BadRequestError, create_client
from liter_llm._internal_bindings import ChatCompletionRequest, DefaultClient, EmbeddingRequest


def _serve(listener: socket.socket, seen: list[str]) -> None:
    while True:
        try:
            conn, _ = listener.accept()
        except OSError:
            return
        with conn:
            conn.settimeout(2)
            try:
                seen.append(conn.makefile("rb").readline().decode().strip())
                conn.sendall(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            except OSError:
                continue


@pytest.fixture
def spy(monkeypatch: pytest.MonkeyPatch) -> Iterator[list[str]]:
    seen: list[str] = []
    listener = socket.socket()
    listener.bind(("127.0.0.1", 0))
    listener.listen()
    threading.Thread(target=_serve, args=(listener, seen), daemon=True).start()
    url = f"http://127.0.0.1:{listener.getsockname()[1]}"
    for var in ("HTTPS_PROXY", "HTTP_PROXY", "https_proxy", "http_proxy"):
        monkeypatch.setenv(var, url)
    for var in ("NO_PROXY", "no_proxy", "ALL_PROXY", "all_proxy"):
        monkeypatch.delenv(var, raising=False)
    yield seen
    listener.close()


def _chat(model: str) -> ChatCompletionRequest:
    return ChatCompletionRequest.from_json(json.dumps({"model": model, "messages": [{"role": "user", "content": "hi"}]}))


def _embed(model: str) -> EmbeddingRequest:
    return EmbeddingRequest.from_json(json.dumps({"model": model, "input": "hi"}))


async def _call_chat(client: DefaultClient, model: str) -> object:
    return await client.chat(_chat(model))


async def _call_embed(client: DefaultClient, model: str) -> object:
    return await client.embed(_embed(model))


UNROUTED = ["snowflake", "watsonx", "gigachat", "oci", "databricks"]
GATEWAYS = ["custom", "hosted_vllm", "litellm_proxy", "openai_like"]


@pytest.mark.parametrize("provider", UNROUTED + GATEWAYS)
def test_provider_without_route_is_refused_and_sends_nothing(provider: str, spy: list[str]) -> None:
    client = create_client(api_key="key-for-another-vendor", max_retries=0, timeout_secs=8)
    for call in (_call_chat, _call_embed):
        with pytest.raises(BadRequestError) as info:
            asyncio.run(call(client, f"{provider}/m"))
        assert provider in str(info.value)
        assert "base_url" in str(info.value)
    assert spy == []


def test_openai_model_reaches_the_spy(spy: list[str]) -> None:
    client = create_client(api_key="key", max_retries=0, timeout_secs=8)
    with pytest.raises(Exception) as info:  # noqa: B017, PT011 - the spy answers 502
        asyncio.run(_call_chat(client, "gpt-4o"))
    assert not isinstance(info.value, BadRequestError), info.value
    assert any("api.openai.com" in line for line in spy), spy


def test_client_hinted_at_a_provider_without_route_is_refused(spy: list[str]) -> None:
    with pytest.raises(BadRequestError, match="base_url"):
        create_client(api_key="key", model_hint="snowflake/arctic")
    assert spy == []


def test_provider_without_route_is_sent_to_its_base_url(spy: list[str]) -> None:
    client = create_client(api_key="key", base_url="http://127.0.0.1:9", max_retries=0, timeout_secs=8)
    with pytest.raises(Exception) as info:  # noqa: B017, PT011 - the spy answers 502
        asyncio.run(_call_chat(client, "snowflake/arctic"))
    assert not isinstance(info.value, BadRequestError), info.value
    assert any("127.0.0.1:9" in line for line in spy), spy
