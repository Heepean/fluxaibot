# FluxAiBot v1.1.1

## Fixes and improvements

- Fixed Hugging Face search warnings for GGUF repositories whose variants are all larger than the selected size filter. These models are now filtered out without being reported as missing GGUF files.
- Kept a clear warning when repository metadata contains no downloadable GGUF variants.
- Expanded localization across all ten supported languages, including dynamic status and download messages, Hugging Face warnings, model actions, and external-app compatibility notes.
- Localized dynamically displayed messages before they are shown or added to the activity log.

## Verification

- `node --check ui/app.js`
- `node --check ui/i18n.js`
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib model_hub::tests`

Technical detail supplied by the operating system or external services may remain in its original language.
