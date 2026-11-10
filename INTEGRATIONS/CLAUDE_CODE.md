# Claude Code integration

## Confirmed documented route
LM Studio documents Claude Code connectivity through its Anthropic-compatible `POST /v1/messages` endpoint. Its guide uses `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, and `CLAUDE_CODE_ATTRIBUTION_HEADER`; if server authentication is enabled, the token must be the LM Studio API token.

## Windows-specific implementation
Claude Code on native Windows requires Git for Windows / Git Bash (or WSL); detect the supported installation path and do not assume PowerShell alone is a supported shell environment.

For a Hub-launched Claude Code process, set environment variables only on that child process. Do not persist machine-wide variables by default. Example PowerShell values for a user-run terminal (not a persistent Hub action):

```powershell
$env:ANTHROPIC_BASE_URL = "http://127.0.0.1:12345"
$env:ANTHROPIC_AUTH_TOKEN = "<Hub-local-token>"
$env:CLAUDE_CODE_ATTRIBUTION_HEADER = "0"
claude
```

Actual model naming and command arguments must be validated against the installed Claude Code and backend version.

## Hub implementation
- Implement Anthropic Messages API, not merely OpenAI Chat Completions.
- Test message content blocks, system/developer handling, tool use, streaming events, stop reasons, token accounting and error mapping.
- Do not bypass Anthropic authentication, usage controls, or vendor policy. An endpoint override is a routing feature, not an authorization bypass.
- If user runs Claude Code through WSL, Windows localhost routing and path translation need a separate tested adapter; do not assume host networking behavior.

## Status
**Supported conditionally** when the Hub endpoint implements the Messages API and the installed Claude Code mode allows the endpoint override. Native Windows requires supported shell setup; WSL needs separate validation.

## Official documentation
- https://lmstudio.ai/docs/integrations/claude-code
- https://docs.anthropic.com/en/docs/claude-code/getting-started
- https://docs.anthropic.com/en/docs/claude-code/llm-gateway
