# СТАРТ ДЛЯ ИИ-РАЗРАБОТЧИКА — Local AI Hub

Прочитай этот файл, затем полностью прочитай `AGENTS.md`, `MASTER_SPEC.md`, `ARCHITECTURE.md`, все документы `ENGINE_SPECS/` и `INTEGRATIONS/`. Не начинай сразу писать интерфейс.

## Цель продукта
Создать настоящее Windows-приложение (Windows 10/11 x64) на Rust с GUI на Tauri 2, которое управляет локальным runtime для GGUF и подключает поддерживаемые инструменты разработки к локальной модели через документированные протоколы. В релизе не должно быть заглушек, псевдо-кнопок и ложного статуса «Подключено».

## Принятые решения
- Core: Rust stable (MSVC). GUI: Tauri 2.
- Runtime MVP: `llama.cpp` `llama-server`, GGUF, CPU fallback обязателен. GPU acceleration только если backend binary реально поддерживает устройство.
- Все серверы по умолчанию слушают `127.0.0.1`, не LAN.
- Интеграции через официальные CLI/API/config mechanisms. Ни одной правки private/undocumented state.
- Конфигурации сторонних программ: backup → diff → explicit approval → atomic write → validation → live verification → rollback.
- Нельзя обходить vendor auth, лицензии, подписки или ограничения функций.

## Ключевые особенности интеграций
1. **Claude Code:** нужен Anthropic Messages API (`/v1/messages`). Для native Windows проверить Git for Windows/Git Bash. Переменные endpoint передавать процессу Claude Code без глобальной записи по умолчанию.
2. **Codex:** документированный путь через LM Studio — `codex --oss`, использующий Responses API. Не предполагать, что arbitrary endpoint поддерживается каждым Codex режимом.
3. **VS Code:** текущая документация описывает Custom Endpoint для Chat Completions, Responses и Anthropic Messages. BYOK/local не означает local inline completions или замену всех Copilot функций.
4. **OpenCode:** public custom provider; выбирать правильный provider package под Chat Completions или Responses. Версионировать JSON/JSONC schema.
5. **LM Studio:** `lms` CLI и публичный HTTP API. Проверять `lms --help` и version; не полагаться на фиксированный путь установки.
6. **Bionic:** отдельное приложение. Public docs подтверждают его собственную local model library и remote/cloud модельные режимы, но не универсальное подключение arbitrary внешнего endpoint. Пока только безопасный manual route.

## Первая задача — Phase 0, без реализации GUI
Создай `docs/COMPATIBILITY_SPIKE.md` и небольшой тестовый harness, который доказывает каждый интеграционный маршрут на чистой Windows-машине. Зафиксируй версии всех приложений, официальные ссылки, точную последовательность действий, запросы и ответы, поддержку streaming/tool calls, ограничения и pass/fail.

Проверь минимум:
- `llama-server` CPU launch, readiness, `/v1/models`, chat completion, SSE stream, cancellation.
- Responses API roundtrip и tool call; нужен для Codex/Responses clients.
- Anthropic Messages API roundtrip, streaming и tool call; нужен для Claude Code.
- OpenCode custom provider config: создать, validate, запустить тестовый запрос, удалить только Hub-owned provider, убедиться, что прочие настройки сохранены.
- VS Code Custom Endpoint в поддерживаемом harness; отдельно проверить agent tool calling и подтвердить, что inline completions не заявляются как локально подключённые.
- LM Studio CLI discovery, start/status/stop, model list/load; не останавливать чужие процессы.
- Bionic: исследовать только публичную документацию/UI; не reverse-engineer private DB/API.

Если интеграция не подтверждается, зафиксируй `Partial`, `ManualActionRequired` или `Unsupported`; не подменяй её симуляцией.

## После Phase 0
Представь пользователю результаты compatibility spike и перечень реально поддерживаемых функций. Далее реализуй Phase 1 runtime supervisor и API; только затем UI. После каждого вертикального среза запускай fmt, clippy, unit tests и интеграционные проверки.

## Definition of Done
- Нет незавершённых кнопок и пустых обработчиков в поставляемом функционале.
- Нет заглушек и «успешных» сообщений без проверки.
- Все процессы child-supervised, завершаются и не оставляют orphan processes.
- Все операции ограничены timeout и отменяемы.
- Все изменяемые конфиги имеют резервную копию и проверенный откат.
- На чистых Windows 10 и 11 пройдены P0 из `TEST_PLAN.md`.
