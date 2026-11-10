# Performance & Compatibility

## Performance principles
- Keep UI responsive; model loading runs outside UI thread.
- Do not claim higher inference speed from UI optimizations.
- Show measured tokens/sec only from actual timed generation, with model, context, backend and settings attached.
- Separate load time, prompt processing, generation speed and first-token latency.
- Memory estimates are estimates; actual usage depends on KV cache, backend, context and offload.
- Avoid automatic maximum GPU offload; provide Auto profile and manual advanced control.

## Compatibility tiers
- Tier A: protocol + end-to-end test passes on pinned version.
- Tier B: documented setup exists but requires user action.
- Tier C: partial features or protocol limitations.
- Tier D: no supported public integration.
Every integration card must display tier and tested version.

## Initial expected tiers
- LM Studio: A for documented CLI/API operations, subject to version checks.
- OpenCode: A/B for custom OpenAI-compatible provider when API requirements pass.
- Claude Code: A/B for Anthropic-compatible Messages endpoint and supported usage mode.
- Codex: A for LM Studio OSS mode; custom Hub endpoint conditional on Responses API/provider support.
- VS Code: B/C because provider extension and feature-specific limitations apply.
- Bionic: D until public config/API integration is verified.
