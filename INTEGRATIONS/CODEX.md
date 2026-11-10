# OpenAI Codex integration

## Confirmed documented route
LM Studio documents using Codex with its OpenAI-compatible `POST /v1/responses` endpoint. The documented flow is to start the LM Studio local server (default port 1234) and run `codex --oss`; a model can be selected with `-m <model-id>`. Model and server context capacity matter for agent workflows.

## Hub implementation
- Detect Codex CLI/app and record its version. Do not assume CLI and app expose identical configuration surfaces.
- For LM Studio backend, offer the documented OSS flow; check `lms server status`/health and verify model id before launching Codex.
- For Hub's own `llama-server` backend, do not assume `codex --oss` accepts an arbitrary endpoint. A custom provider route is allowed only if the exact current Codex version officially documents it.
- Implement and test the Responses API contract before advertising direct Codex support: input/output item structure, function/tool calls, streaming events, reasoning fields, cancellation and errors. Chat Completions alone is insufficient.
- Launch with a scoped child-process environment and avoid global environment changes. Preserve Codex auth/profile state.
- Do not intercept ChatGPT sign-in, copy private credentials, or claim that a ChatGPT subscription authorizes arbitrary local API usage.

## Status
**Supported via the documented LM Studio OSS flow.** Direct Hub-runtime support is conditional on the detected Codex version and a verified provider mechanism.

## Official documentation
- https://lmstudio.ai/docs/integrations/codex
- https://lmstudio.ai/docs/developer/openai-compat/responses
