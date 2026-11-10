# UI Specification

## Navigation
- Dashboard
- Models
- Runtime
- Integrations
- Profiles
- Diagnostics
- Settings

## Dashboard
Show selected model, backend, runtime state, API endpoint, context, estimated RAM/VRAM, actual process state, and quick Start/Stop.
No fabricated live GPU metrics. If telemetry source is unavailable, show “Unavailable”.

## Models
List discovered GGUF with file size, quantization if parseable, architecture, context metadata and hash status. Actions: Add folder, Refresh, Inspect, Select. Delete is out of scope for MVP.

## Integrations
Cards for Claude Code, Codex, VS Code, OpenCode, LM Studio, Bionic. Each card displays detected version, integration mode, protocol requirement, current state, last verification time, Connect/Repair/Disconnect.
Connect All must show a review screen with per-app changes before apply.

## Profiles
Save runtime + model + integration selections. Switching profile requires clear explanation of runtime restart and config impact.

## Diagnostics
Structured checks, logs, copy diagnostic bundle. Redact secrets, usernames and model paths by default. Export raw logs only with consent.

## UX
- Dark/light theme.
- Keyboard accessible.
- Clear progress and cancellation.
- No blocking dialogs for routine operations.
- Error messages include code, explanation and next step.
- Never show green success state before verification.
