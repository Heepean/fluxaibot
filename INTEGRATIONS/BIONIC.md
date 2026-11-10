# LM Studio Bionic integration

## Product identity
Bionic means **LM Studio Bionic**, a separate agent application from LM Studio. It must not be treated as a synonym for the LM Studio desktop app or its `lms` CLI.

## Confirmed documented behavior
Current Bionic documentation describes sessions using local models in Bionic's local model library, cloud models, or remote models on another device through LM Link. The public docs consulted do not establish a general supported configuration API for pointing Bionic to an arbitrary third-party OpenAI-compatible endpoint such as a separately hosted Hub gateway.

## Hub implementation
- Detect Bionic and its version only; do not assume it shares LM Studio's configuration database or CLI.
- Provide an **Open Bionic** action and a concise manual route to select a locally available model in Bionic.
- If future official docs expose a supported external provider/API, implement it as a version-gated adapter with config backup and end-to-end tests.
- Do not edit undocumented settings files, embedded databases, private endpoints, or automate clicks by screen coordinates.
- Do not claim that a model served by Hub/llama-server can automatically appear in Bionic's model picker unless that specific path has been documented and tested.

## Status
**Manual / not guaranteed for arbitrary Hub endpoint.** Bionic can use local models through its supported local model workflow; generic third-party endpoint integration remains unverified.

## Official documentation
- https://lmstudio.ai/docs/bionic
- https://lmstudio.ai/docs/bionic/models
- https://lmstudio.ai/docs/bionic/models/download-local-models
