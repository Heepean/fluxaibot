# Security Requirements

## Threat model
Protect local model files, project files, API tokens, third-party application configs and local API from other local processes/network clients.

## Mandatory controls
- Bind to `127.0.0.1` by default.
- Random bearer token; never use a hard-coded token.
- Store secrets using Windows Credential Manager or DPAPI; redact from logs and crash reports.
- No remote bind without explicit warning, authentication and firewall guidance.
- Strict request size/time limits.
- Validate all file paths; defend against path traversal, symlink/reparse-point escape and TOCTOU.
- Spawn processes without shell interpolation.
- Allowlist runtime executable and arguments.
- Download runtime/model metadata only from configured trusted sources; verify SHA-256 and Authenticode signature when available.
- Do not run downloaded scripts.
- Config writes: backup, atomic replace, ACL preservation where feasible.
- No telemetry by default; opt-in diagnostics must disclose contents.
- Never read project source code or prompts unless needed for a user-requested feature.
- Do not log prompt/completion content by default.

## Windows
- Use per-user data directory.
- Avoid requiring Administrator.
- Do not add Defender exclusions automatically.
- Do not change firewall rules silently.
- Do not register auto-start without consent.
