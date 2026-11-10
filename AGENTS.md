# AGENTS.md — mandatory instructions for AI developers

## Mission
Implement Local AI Hub exactly according to the specification. Prioritize a working, verifiable Windows product over feature count.

## Before coding
1. Read every file in this package.
2. Inspect current official docs for all six target apps and record URLs, access date, app version and relevant config schema.
3. Produce a compatibility matrix. Explicitly flag unsupported or partial integrations.
4. Create a task plan with dependencies and acceptance tests.
5. Ask the user before changing any unresolved architectural decision or making destructive changes.

## Non-negotiable rules
- Rust is the core language.
- Windows 10/11 x64 is the target.
- No images or image generation are needed.
- No placeholder implementations, fake buttons, TODO-only paths or simulated success.
- Never claim integration is working until an end-to-end verification succeeds.
- Never bypass subscription/authentication restrictions or undocumented app internals.
- Never overwrite user config without backup, diff and rollback.
- Never run commands through shell strings when direct process args are possible.
- Do not silently bind API to LAN.
- Do not log secrets, prompts or source code by default.
- Do not invent APIs, config keys, CLI flags or file paths.
- Keep changes small, compile and test after each coherent step.
- Run formatter, clippy and tests before declaring completion.
- Preserve unrelated settings and user data.

## Coding standards
- Typed errors and explicit state machines.
- Async operations must be cancellable and bounded by timeouts.
- Every child process must be supervised and reaped.
- Every config write must be transactional.
- Every feature must have unit and integration tests.
- Pin dependencies and runtime versions; document update policy.
- Prefer stable Rust and maintained crates; justify every native dependency.

## Required workflow
For each task:
1. State expected behavior and acceptance criteria.
2. Inspect responsible code.
3. Implement smallest complete slice.
4. Add tests.
5. Run targeted tests.
6. Run full relevant checks.
7. Report files changed, commands run, test results and known limitations.

## Stop conditions
Stop and ask user if:
- a target app has no documented supported integration;
- a protocol implementation would require undocumented behavior;
- config parsing cannot preserve user data;
- runtime binary provenance/signature cannot be verified;
- a feature requires admin rights or broad system changes;
- the requested behavior conflicts with vendor terms or security.
