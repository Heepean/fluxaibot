# OpenCode integration

## Confirmed documented route
OpenCode supports custom providers and local runtimes, including LM Studio and llama.cpp. Its current provider documentation describes provider IDs, model IDs, runtime packages, base URLs and API-compatible provider packages. The exact config schema has evolved; the adapter must follow the schema for the detected OpenCode version rather than blindly writing a sample from an older guide.

## Hub implementation
- Detect OpenCode executable, version, install method and active config path.
- Retrieve/validate the schema associated with that version using an official or bundled, version-pinned schema. Do not rely on online fetch at every startup.
- Prefer JSONC-aware parsing when comments are allowed. Preserve comments and unrelated keys; if the parser cannot preserve them safely, present a snippet/manual action rather than rewriting the whole file.
- Add a Hub-owned provider key only; do not replace the user's selected provider/model. Use the model id actually returned by `GET /v1/models`.
- Choose provider runtime package based on protocol: Chat Completions-compatible or Responses-compatible. Never use the wrong package merely because both are “OpenAI-like”.
- Add a model capability declaration only after probing tool calling, context and supported modalities.
- Backup, diff, atomic write, schema validation, launch verification and rollback are mandatory.

## Status
**Supported** through documented custom provider configuration, subject to detected version, protocol match and tool-call verification.

## Official documentation
- https://opencode.ai/docs/providers
