# Build & Release

## Dev prerequisites
- Rust stable MSVC toolchain.
- Visual Studio Build Tools: Desktop development with C++, including the Windows x64 SDK resource compiler.
- WebView2 Runtime.
- Node.js LTS only for frontend build tooling.
- Tauri CLI 2.12.4 (`cargo install tauri-cli --version 2.12.4 --locked`).

## Build
- After cloning, install Git LFS and fetch the embedded runtime binaries with `git lfs pull`.
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- frontend typecheck/lint/test
- `.\build-release.ps1` builds a timestamped `dist\release-<date>` directory containing the portable EXE, NSIS setup EXE, and `SHA256SUMS.txt`.
- Verify both artifacts on a clean Windows VM.
- Run `Publish-to-GitHub.bat` to verify the latest release files, sign in through GitHub's official browser flow if needed, push `main` and its LFS assets, and publish v1.0.0 to the configured repository.

## Distribution
- CPU, Vulkan, and CUDA `llama-server` executables and required CUDA/MSVC runtime DLLs are embedded in the application binary and extracted to `%LOCALAPPDATA%\FluxAiBot\runtime` only after SHA-256 validation.
- Portable and installed builds store application state under `%LOCALAPPDATA%\FluxAiBot`; GGUF model files are not included.
- The portable executable is a single-file distribution; Windows WebView2 Runtime remains a system prerequisite.
- Sign installer and binaries with Authenticode when certificate available.
- Publish `SHA256SUMS.txt` with each GitHub Release.
- Include licenses/notices for Tauri, runtime and all bundled dependencies.
- Runtime updates ship with the application and remain pinned by `runtime-manifest.json`.
