# Implementation Roadmap

## Phase 0 — compatibility spike
Before GUI implementation, build a throwaway test harness for:
- llama-server CPU on Windows.
- OpenAI Chat Completions streaming.
- Responses API compatibility required by Codex.
- Anthropic Messages API compatibility required by Claude Code.
- OpenCode custom provider config patch/validation.
- Claude Code env-scoped launch.
- Codex OSS + LM Studio documented workflow.
- VS Code provider extension workflow.
- Bionic public integration investigation.
Deliverable: compatibility matrix with evidence and pinned app versions.

## Phase 1 — core runtime
Model scanner, runtime supervisor, lifecycle state machine, logs, health checks, basic API smoke test.

## Phase 2 — API gateway
OpenAI-compatible endpoints, streaming, cancellation, tool calls; then separate Anthropic Messages and Responses API support with contract tests.

## Phase 3 — GUI
Dashboard, model browser, runtime profiles, diagnostics, error states.

## Phase 4 — integrations
Implement adapters in this order:
1. LM Studio (CLI/API).
2. OpenCode (custom provider).
3. Claude Code (Anthropic endpoint + scoped env).
4. Codex (documented OSS mode; custom route only after verification).
5. VS Code (provider extension guidance/optional companion extension).
6. Bionic (only supported public workflow).

## Phase 5 — reliability/security
Config transactions, rollback, token storage, path validation, crash recovery, port conflict tests.

## Phase 6 — release
Windows 10/11 clean install, P0 test suite, signing, SBOM, user guide, release candidate. No release if P0 gates fail.

## Definition of done
No TODO placeholders in shipped code paths, no fake integration success, no unimplemented button, no swallowed errors, no unverified API claims.
