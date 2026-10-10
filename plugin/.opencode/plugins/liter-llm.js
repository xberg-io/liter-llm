// AI-RULEZ :: GENERATED FILE — DO NOT EDIT
// Content-Hash: blake3:864d6c9a7cda07ab38a0f5d3e8cd5dd20ff22cfa1136469e1b5b294433f4b819
// Source-Hash: blake3:3cb955aedbbbd51d9b5f60a411d90b9386534c5f14bf20a056bef60f7dace49e
// Schema-Version: v1

/**
 * OpenCode-specific plugin entrypoint.
 *
 * ai-rulez copies this source module into the generated OpenCode package.
 * Shared skills, commands, agents, and MCP configuration belong in their normal
 * `.ai-rulez` sources; add only OpenCode-specific tools or hooks here.
 *
 * To extend this plugin:
 * 1. Import `tool` from `@opencode-ai/plugin`.
 * 2. Define tool arguments with `tool.schema` and validate every external
 * input.
 * 3. Return the OpenCode hooks object from this function.
 * 4. Preview with `ai-rulez generate --plugin --dry-run` before regenerating.
 *
 * Pass subprocess arguments as an array. Never interpolate external input into
 * a shell command.
 */
const LiterLlmPlugin = () => ({});

export default LiterLlmPlugin;
