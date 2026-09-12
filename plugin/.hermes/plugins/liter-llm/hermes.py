# AI-RULEZ :: GENERATED FILE — DO NOT EDIT
# Content-Hash: blake3:ea67a2034fe7813e671d3946523d836f6a1d77bc2fa96f766f838d77aa9068a4
# Source-Hash: blake3:4c4c59be6bcc57aad4a50d69304767696cce151dad29c0a5b25341e1f046769f
# Schema-Version: v1

"""Hermes adapter for liter-llm.

This generated no-op keeps the plugin loadable without inventing runtime behavior.
To add Hermes tools, hooks, commands, or other registrations:

1. Create .ai-rulez/hermes/index.py.
2. Implement register(ctx) in that user-owned source file.
3. Run ai-rulez generate --plugin.

Project-local Hermes plugins are trusted code. Enable them explicitly with
HERMES_ENABLE_PROJECT_PLUGINS=true and validate all external input.
"""


def register(ctx):
    """Register this plugin with Hermes Agent."""
    del ctx
