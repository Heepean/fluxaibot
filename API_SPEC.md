# Local API Contract

Default URL: `http://127.0.0.1:<port>`. Port default 12345, configurable. Authentication: Bearer token required by default for clients that support it; adapter-specific auth behavior documented.

## Endpoints
- `GET /health` — process health only.
- `GET /v1/models` — models actually loaded/served.
- `POST /v1/chat/completions` — OpenAI chat subset; streaming SSE.
- `POST /v1/responses` — only after full Responses protocol implementation.
- `POST /v1/messages` — Anthropic Messages compatibility only after full implementation.
- `GET /hub/v1/capabilities` — Hub capability document.
- `GET /hub/v1/status` — runtime and model state, no secrets.

## Model IDs
Stable Hub model ID generated from model metadata/path hash, with user-overridable alias. Adapter maps Hub ID to backend model id.

## Errors
Every error includes `error.type`, `error.code`, `error.message`, `request_id`. Do not include filesystem paths or secrets unless diagnostics explicitly export them with user consent.

## Compatibility policy
An endpoint is not considered supported because the HTTP route exists. It must pass protocol conformance tests for required request fields, response fields, streaming, stop/cancel, tool calls and error behavior.
