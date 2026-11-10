# Compatibility spike

Research date: 2026-11-10. Product behavior and vendor documentation can change; re-check the linked sources before shipping.

## Local environment

| App | Detected version | Notes |
|---|---|---|
| LM Studio | GUI version not verified; `lms.exe` file metadata reports 1.3.3, and `lms --version` reports CLI commit `43d4b26` | The executable metadata identifies its product as Bun, so it is not evidence of the LM Studio desktop version. |
| Claude Code | Not detected | No version available for native Windows or WSL testing. |
| Codex | Not detected | No version available for OSS workflow testing. |
| VS Code | 1.145.2 | Detected through `code --version`. |
| OpenCode | Not detected | No version available for provider/config testing. |
| Bionic | Not detected | Separate product from LM Studio; no version available. |
| llama.cpp `llama-server` | Bundled CPU build 0.9.0, source commit `cd959bae99ff4bed22bef2604f45241c09bbb8c8` | Built locally from the official upstream source. SHA-256 is recorded in `src-tauri/resources/runtime-manifest.json`; no vendor Authenticode signature is available for this locally built binary. |

## Compatibility matrix

The matrix separates documented vendor paths from paths implemented and end-to-end verified by this application. No integration currently has an end-to-end pass result.

| Target | Documented path and relevant schema | Current assessment |
|---|---|---|
| LM Studio | `lms` CLI (`lms server start`, `lms server status`, `lms server stop`) and local HTTP APIs. OpenAI-compatible endpoints include `/v1/models`, `/v1/chat/completions`, and `/v1/responses`; Anthropic-compatible endpoint is `/v1/messages`. | FluxAiBot only detects LM Studio. It does not configure or manage its server; compatibility with the bundled gateway is not verified. |
| Claude Code | Anthropic Messages API at `/v1/messages`; documented environment variables include `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, and `CLAUDE_CODE_ATTRIBUTION_HEADER`. | Not integrated: the bundled gateway does not implement Anthropic Messages. App not installed; no live test. |
| Codex | LM Studio documents `codex --oss` against its `/v1/responses` endpoint, with optional `-m <model-id>`. | Not integrated: the bundled gateway does not implement Responses and FluxAiBot does not configure Codex. App not installed. |
| VS Code | The Language Models editor provides a Custom Endpoint workflow; the selected API format and model capabilities determine compatibility. This does not replace Copilot features such as local inline completions. | Detection only. Custom Endpoint setup, chat, streaming, and tool calls are not verified; no feature is reported as supported by FluxAiBot. |
| OpenCode | Current provider config is JSON/JSONC (`opencode.json` or `opencode.jsonc`). A custom provider uses `providers.<id>` with `name`, `env`, `package`, `settings.baseURL`, and `models`. | Not integrated: provider configuration is not implemented or tested. App not installed. |
| Bionic (LM Studio) | The Bionic model picker documents local models, LM Studio Secure Cloud, and remote models through LM Link. | No documented arbitrary third-party model endpoint or external config API found. FluxAiBot cannot claim an automatic connection. App not installed. |
| Ollama | Ollama documents an OpenAI-compatible server API, including `/v1/chat/completions`; its examples configure OpenAI clients to call Ollama at `http://localhost:11434/v1/`. | Ollama is documented as an inference server/API provider, not as a client that can be configured to consume FluxAiBot. FluxAiBot does not currently delegate inference to Ollama. |
| Comfy Desktop | Comfy documents Comfy MCP for connecting supported AI agents to ComfyUI workflows. The in-app Comfy Agent is generally available in Cloud; local support for Comfy Desktop is documented as coming soon. | The documented Comfy integration is MCP/workflow tooling, not a configurable LLM endpoint. FluxAiBot does not implement MCP; automatic model connection is unsupported. |

## Official sources

All sources below were checked on 2026-11-10.

| Target | Official documentation |
|---|---|
| LM Studio | [CLI](https://lmstudio.ai/docs/cli), [server](https://lmstudio.ai/docs/developer/core/server), [OpenAI-compatible APIs](https://lmstudio.ai/docs/developer/openai-compat), [Responses](https://lmstudio.ai/docs/developer/openai-compat/responses), [Anthropic Messages](https://lmstudio.ai/docs/developer/anthropic-compat/messages) |
| Claude Code | [LM Studio integration](https://lmstudio.ai/docs/integrations/claude-code), [Anthropic gateway documentation](https://code.claude.com/docs/en/llm-gateway) |
| Codex | [LM Studio integration](https://lmstudio.ai/docs/integrations/codex), [Responses API compatibility](https://lmstudio.ai/docs/developer/openai-compat/responses) |
| VS Code | [Language models and custom endpoints](https://code.visualstudio.com/docs/agent-customization/language-models), [Language Model API](https://code.visualstudio.com/api/extension-guides/ai/language-model) |
| OpenCode | [Providers and custom provider configuration](https://opencode.ai/docs/providers) |
| Bionic (LM Studio) | [Product overview](https://lmstudio.ai/docs/bionic), [model selection](https://lmstudio.ai/docs/bionic/models), [LM Link API](https://lmstudio.ai/docs/developer/core/lmlink) |
| Ollama | [OpenAI-compatible API](https://docs.ollama.com/api/openai-compatibility) |
| Comfy Desktop | [Comfy MCP](https://docs.comfy.org/agent-tools/mcp), [Comfy Agent availability](https://docs.comfy.org/agent-tools/in-app-agent) |
| Hugging Face model hub | [Hub REST API](https://huggingface.co/docs/hub/api), [Download files](https://huggingface.co/docs/huggingface_hub/guides/download) |

## Hugging Face model catalog

| Integration | Documented API/config | Current assessment |
|---|---|---|
| Hugging Face public GGUF search and download | `GET https://huggingface.co/api/models?filter=gguf&pipeline_tag=text-generation`; repository metadata from `/api/models/{repo_id}?blobs=true`; files from `/resolve/{revision}/{path}`. Public requests only; no token or gated-access bypass. | Implemented in the **Download AI** tab, including GGUF variant/shard selection, size and RAM filters, cancellable staged downloads, and size/SHA-256 verification when LFS metadata provides a digest. A successful full-size model download and inference have not been verified in this environment. |

The official [llama.cpp v0.9.0 release](https://github.com/ggml-org/llama.cpp/releases/tag/v0.9.0) points to commit `cd959bae99ff4bed22bef2604f45241c09bbb8c8`. Its GitHub release metadata contains `nightly-tag.txt` but no Windows runtime binary or published checksum/signature for one. The official release page therefore does not provide a prebuilt `llama-server.exe` that can be installed and verified as-is.

## Verification gates

No pass result should be shown until the corresponding live test has passed on a recorded app/runtime version.

1. **Passed:** bundle source-built CPU `llama-server` 0.9.0 from the pinned official commit; verify its recorded SHA-256 before launch. The locally built executable has no Authenticode signature.
2. Launch a known small GGUF and verify readiness, `/v1/models`, a real non-streaming request, SSE streaming, cancellation, and clean stop.
3. Verify gateway authentication and error behavior; ensure a request with no active model returns an error rather than generated placeholder text.
4. Test Claude Messages and Codex Responses protocol contracts, including streaming and tools, against exact installed versions before advertising those integrations.
5. Test OpenCode provider config round-trip in a temporary profile, preserving unrelated keys and removing only the Hub-owned provider.
6. Complete the documented VS Code Custom Endpoint flow and test actual chat/tool behavior; keep inline completion support separate.
7. Test `lms` CLI lifecycle without terminating a server not started by Hub. Treat Bionic as manual for arbitrary endpoints.

## Current blocker

The runtime is bundled, but this checkout has no GGUF model. Readiness with a real model, actual completion, SSE streaming, cancellation, and clean model shutdown have not been verified. Integration adapters currently perform detection only; external application configuration and protocol routes are not implemented.
