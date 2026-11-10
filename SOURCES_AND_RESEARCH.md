# Official sources & research notes

Research checked: **2026-11-09**. Vendor docs and product behavior can change; re-check at implementation time and pin tested versions. No undocumented APIs should be inferred from examples.

## LM Studio / Bionic
- CLI: https://lmstudio.ai/docs/cli
- Local server: https://lmstudio.ai/docs/developer/core/server
- OpenAI-compatible endpoints: https://lmstudio.ai/docs/developer/openai-compat
- Responses endpoint: https://lmstudio.ai/docs/developer/openai-compat/responses
- Codex integration: https://lmstudio.ai/docs/integrations/codex
- Claude Code integration: https://lmstudio.ai/docs/integrations/claude-code
- Bionic overview: https://lmstudio.ai/docs/bionic
- Bionic model selection: https://lmstudio.ai/docs/bionic/models
- Bionic local model download: https://lmstudio.ai/docs/bionic/models/download-local-models

## OpenCode
- Provider documentation: https://opencode.ai/docs/providers

## Visual Studio Code
- Language models / BYOK / Custom Endpoint / on-device models: https://code.visualstudio.com/docs/agent-customization/language-models
- Extension Language Model API: https://code.visualstudio.com/api/extension-guides/ai/language-model

## Claude Code
- Getting started and Windows setup: https://docs.anthropic.com/en/docs/claude-code/getting-started
- LLM gateway and endpoint routing: https://docs.anthropic.com/en/docs/claude-code/llm-gateway

## Codex
- LM Studio's current documented Codex workflow: https://lmstudio.ai/docs/integrations/codex
- Codex/Responses compatibility source: https://lmstudio.ai/docs/developer/openai-compat/responses
- Implementation rule: verify the actual Codex CLI/app docs for the detected build before claiming that an arbitrary custom endpoint is supported.

## Tauri
- Windows prerequisites: https://tauri.app/start/prerequisites/

## Important findings
1. Codex local integration is explicitly documented by LM Studio using `/v1/responses` and `codex --oss`. Do not generalize this to every arbitrary backend.
2. Claude Code integration is explicitly documented by LM Studio using Anthropic Messages (`/v1/messages`); Windows native shell requirements and WSL networking require their own tests.
3. OpenCode has documented custom provider configuration. Match the package to the API protocol and detected config schema version.
4. Current VS Code docs describe Custom Endpoint providers supporting Chat Completions, Responses and Anthropic Messages APIs for chat/agent scenarios. This does not enable local BYOK inline completions; some Copilot-backed features remain service-dependent.
5. Bionic is separate from LM Studio. Public docs describe local/cloud/remote model selection but do not establish a generic third-party endpoint configuration API. Treat it as manual until proven otherwise.
