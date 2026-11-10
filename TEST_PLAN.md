# Test Plan

## P0 release gates
1. Clean install on Windows 10 x64 and Windows 11 x64.
2. Start CPU runtime with small known GGUF; health and model listing pass.
3. Chat Completions non-streaming and streaming pass.
4. Cancellation stops generation and releases request resources.
5. Runtime crash is detected and shown; app remains usable.
6. Start/stop repeated 100 cycles without orphan processes.
7. Existing port conflict does not kill/modify foreign process.
8. Claude Messages protocol conformance if Claude integration is advertised.
9. Codex Responses API / OSS route verification for the exact supported mode.
10. OpenCode provider config round-trip preserves unrelated keys.
11. VS Code path never claims universal Copilot/inline integration.
12. LM Studio CLI version detection and server lifecycle tested.
13. Bionic is marked manual/unsupported unless public integration is verified.
14. Config backup, apply, external edit detection and rollback pass.
15. No secrets in logs or exported diagnostics.
16. Bind address remains loopback after restart/update.
17. GPU failure fallback to CPU works only after user approval.

## Test levels
- Unit: parsers, config patchers, state machine, model metadata.
- Contract: OpenAI/Anthropic/Responses protocol fixtures.
- Integration: actual runtime binary and temporary user profile.
- UI: smoke tests for all navigation and cancellation states.
- Manual hardware matrix: CPU-only, AMD Vulkan, NVIDIA CUDA/Vulkan, Intel supported backend.
- Regression: test against pinned versions and latest supported versions.

## Release evidence
Each P0 test must produce machine-readable result, app version, runtime version, OS build, logs and pass/fail. A skipped test blocks claims of support for that feature.
