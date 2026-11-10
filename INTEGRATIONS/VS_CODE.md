# Visual Studio Code integration (current documented paths)

## What is supported
Current VS Code documentation describes a **Custom Endpoint** provider in the Language Models editor. It supports Chat Completions, Responses, and Anthropic Messages API formats. A local endpoint can therefore be configured for the VS Code chat/agent experience when it implements a matching protocol and the model meets the required tool-calling/capability requirements.

This is not the same as replacing every Copilot feature. BYOK/local providers do not provide local inline code completions through the standard BYOK flow. Some features, including semantic search and embedding-dependent or Copilot-service features, may still require GitHub/Copilot services.

## Hub implementation
1. Detect VS Code stable and Insiders separately, including executable path and version.
2. Prefer a documented VS Code UI workflow: open `Chat: Manage Language Models` → `Add Models` → `Custom Endpoint`; choose the actual protocol supported by Hub.
3. Do not write undocumented internal VS Code state or assume that a JSON configuration file is a stable public API. Only automate a file edit where the current official schema explicitly documents it and a round-trip test proves it.
4. A companion extension is optional, separate work. Use public VS Code extension APIs only. It must not impersonate Copilot or claim to supply inline completions unless it implements a real `InlineCompletionItemProvider`.
5. Verify model discovery, a real chat request, streaming if offered, and tool calling if the user selects agent mode.
6. Display feature boundaries in the UI: Chat, Agent tools, Inline completions, Embeddings/Semantic search. Do not collapse these into one green “Connected” state.

## Status
**Supported with documented Custom Endpoint setup for chat/agent sessions, feature-limited.** One-click configuration is allowed only after confirming detected VS Code version, endpoint protocol and exact public configuration mechanism. Otherwise open the documented UI and mark `ManualActionRequired`.

## Official documentation
- https://code.visualstudio.com/docs/agent-customization/language-models
- https://code.visualstudio.com/api/extension-guides/ai/language-model
