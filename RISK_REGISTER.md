# Risk register

| Risk | Impact | Mitigation / release rule |
|---|---|---|
| Different agents expect different API protocols | Agent appears connected but tool calls fail | Implement separate Chat Completions, Responses and Anthropic Messages adapters; protocol conformance tests |
| Codex local mode supports a specific OSS route, not arbitrary endpoint | False claim of universal Codex support | Version-gated workflow; use documented LM Studio route until custom provider documented |
| VS Code features differ (chat, agent, inline, embeddings) | User expects all Copilot features to use local model | Expose feature matrix; test each separately; do not claim local inline completion from BYOK |
| Bionic has no public arbitrary endpoint config | Fragile private-file hack breaks app/user data | Manual-only integration until public API/config contract exists |
| GGUF runtime build lacks AMD Vulkan support | CPU fallback or launch failure | Inspect runtime capabilities, test RX hardware, make CPU fallback explicit |
| GPU VRAM estimate inaccurate due KV cache/fragmentation | OOM during load/generation | Show estimates as estimates; conservative presets and actionable OOM diagnosis |
| External app updates config while Hub is running | Rollback overwrites user changes | Hash-based conflict detection; refuse automatic rollback on drift |
| Existing server already owns selected port | Hub kills/overwrites unrelated process | Identify PID/executable; never terminate unknown process; choose free port |
| Model download tampered or unexpected | Malicious binary/model artifact | Trusted source allowlist, checksum/signature validation, consent, no auto-run |
| Tauri/WebView2 missing or outdated | App fails to launch | Preflight installer check and actionable WebView2 setup |
| Tool-call capable model behaves inconsistently | Coding agent runs broken/unsafe tools | Tool-call round-trip tests; user approval gates remain with agent application |

Risk accepted only with explicit user-visible limitation, telemetry-free diagnostics, and tested recovery.
