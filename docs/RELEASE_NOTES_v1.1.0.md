# FluxAiBot v1.1.0

FluxAiBot is a Windows 10/11 x64 desktop app for local GGUF inference.

## What's new

- The interface now supports English, Russian, Ukrainian, Spanish, Simplified Chinese, German, French, Brazilian Portuguese, Japanese, and Arabic.
- The default language follows the Windows/WebView language, with English as the fallback for unsupported languages.
- Choose and save a language in **Settings → Interface language**. Arabic uses right-to-left layout.
- Core navigation, settings, model-library controls, Hugging Face search controls, API gateway labels, and common statuses are localized. Some detailed technical messages and text supplied by external applications may remain in their original language.

## Included

- One-file portable EXE and a separate NSIS installer.
- Bundled CPU, Vulkan, and CUDA `llama-server` runtimes; runtime files are extracted only after SHA-256 verification.
- Compute-backend preference: CUDA, then Vulkan, then CPU fallback. Active device and backend are shown in the app.
- Public, non-gated GGUF model search and downloads from Hugging Face, with coding/general-purpose filters and an approximate memory estimate.
- Local OpenAI Chat Completions gateway bound to `127.0.0.1:12345`.
- Saved model-folder selection and local application state.

## Requirements and limitations

- Windows 10/11 x64 and WebView2 Runtime are required. No model is included.
- Vulkan inference was verified on an AMD Radeon RX 7700 XT. The CUDA runtime starts and reports its llama.cpp version, but CUDA inference still needs end-to-end verification on an NVIDIA GPU.
- Image and video inference are not supported.
- External application support depends on each tool's documented protocol and version; see [the compatibility notes](COMPATIBILITY_SPIKE.md).
- The installer and portable EXE are not Authenticode-signed.

Download the portable app or installer from the [GitHub Releases page](https://github.com/Heepean/fluxaibot/releases). Verify downloaded files against `SHA256SUMS.txt`.
