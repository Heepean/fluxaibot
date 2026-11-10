# LM Studio integration

## Official capabilities
LM Studio provides the `lms` CLI and a local server. Documented commands include server start/status/stop and model load/unload operations (exact arguments are version-dependent). Its OpenAI-compatible endpoints include `/v1/models`, `/v1/responses`, `/v1/chat/completions`, `/v1/completions` and `/v1/embeddings`; it also documents an Anthropic-compatible Messages endpoint.

## Hub implementation
- Detect `lms` via PATH and documented installation locations; resolve the executable and query its version/help rather than assuming an install path.
- Offer two mutually clear modes: **Hub manages llama.cpp** or **LM Studio manages inference**.
- In LM Studio-managed mode, use documented CLI/API only. Do not read its private database or modify internal model registry.
- Check whether LM Studio's server is already running before starting anything. Never kill a process just because its port is occupied.
- Reconcile loaded model IDs with actual `/v1/models` output.
- Respect LM Studio authentication and bind settings.
- Do not simultaneously start Hub gateway and LM Studio server on the same port.

## Status
**Supported** through public CLI/API, gated by detected version and command capability checks.

## Official documentation
- https://lmstudio.ai/docs/cli
- https://lmstudio.ai/docs/developer/core/server
- https://lmstudio.ai/docs/developer/openai-compat
- https://lmstudio.ai/docs/developer/openai-compat/responses
