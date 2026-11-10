# MASTER SPEC — Local AI Hub

## 1. Цель
Создать компактное нативное Windows-приложение с GUI и EXE, которое:
1. Находит GGUF-модели в выбранных папках.
2. Запускает/останавливает локальный inference runtime.
3. Предоставляет локальный HTTP API на loopback.
4. Настраивает Claude Code, Codex CLI, VS Code, OpenCode, LM Studio и LM Studio Bionic там, где существует поддерживаемый и документированный механизм.
5. Проверяет реальный API-запрос и/или наличие модели после настройки.
6. Безопасно сохраняет и восстанавливает изменённые конфигурации.

## 2. Нецели
- Не создавать собственную IDE или coding agent.
- Не обходить подписки, OAuth, лицензии, квоты или серверную авторизацию.
- Не обещать полную совместимость любой модели с любым агентом.
- Не модифицировать бинарные файлы сторонних приложений.
- Не менять настройки без резервной копии и явного согласия пользователя.
- Не запускать модель на удалённом интерфейсе сети по умолчанию.

## 3. Платформы и упаковка
- Windows 10 x64 и Windows 11 x64.
- Rust stable MSVC target `x86_64-pc-windows-msvc`.
- Tauri 2 для desktop shell.
- WebView2 Evergreen runtime; проверять наличие и давать ссылку/инструкцию установки, не падать молча.
- Поставка EXE installer и portable bundle, если оба варианта проходят тесты.
- Не требовать установленного Python, Docker или отдельной Node.js для конечного пользователя.
- Node.js может быть dev dependency для сборки frontend, но не runtime dependency.

## 4. Пользовательские сценарии
### Первый запуск
- Проверить Windows version/architecture, свободное место, WebView2, права доступа.
- Предложить выбрать каталог моделей и каталог данных приложения.
- Выполнить поиск GGUF только в выбранных каталогах.
- Предложить скачать/установить runtime только из утверждённого источника, показать версию, SHA-256 и лицензию.
- Ничего не запускать и не менять в сторонних программах без подтверждения.

### Запуск модели
- Пользователь выбирает GGUF, backend, контекст, число CPU threads, GPU offload и порт.
- Проверить размер файла, доступность файла, поддерживаемый формат и свободную память (оценка, не гарантия).
- Запустить runtime как дочерний процесс с аргументами, не через shell.
- Дождаться health/readiness endpoint с timeout и cancellation.
- Получить `/v1/models`, отправить короткий тестовый запрос.
- Только после успешных проверок статус `Ready`.

### Connect All
- Обнаружить приложения и версии.
- Показать для каждого: Supported / Partial / Manual required / Not detected / Unsupported.
- Предложить конкретное изменение конфигурации и diff.
- Сделать backup, атомарно применить конфиг, проверить синтаксис/доступность.
- Запустить интеграционный smoke test, где он безопасен и документирован.
- Не считать «записали конфиг» равным «подключение работает».
- В случае ошибки восстановить только файл/ключ, изменённый данной транзакцией.

## 5. Runtime
Рекомендуемый backend первого релиза: `llama.cpp` `llama-server` для GGUF. Приложение управляет бинарником, а не переписывает inference engine на Rust. Причина: зрелая поддержка GGUF, CPU и GPU backend-ов, HTTP API и возможность обновлять runtime отдельно.
- Поддержать CPU backend обязательно.
- GPU backend — только если поставка официально доступного и протестированного Windows build.
- Для AMD GPU предусмотреть Vulkan backend, но не обещать конкретную скорость/совместимость без runtime detection.
- Не встраивать CUDA-only решение как единственный backend.
- LM Studio — внешний backend option, не зависимость собственного режима.
- В будущем можно добавить Ollama/другие backend-адаптеры, не меняя UI/API.

## 6. API
Первый релиз: OpenAI-compatible subset:
- `GET /health`
- `GET /v1/models`
- `POST /v1/chat/completions` (streaming SSE и non-streaming)
- `POST /v1/completions` только если runtime корректно поддерживает endpoint
- `POST /v1/embeddings` только при наличии embedding model и подтверждённой реализации.
- Responses API (`/v1/responses`) — отдельная обязательная capability для Codex; нельзя заявлять поддержку, пока endpoint реально реализован и протестирован.
- Anthropic Messages API (`/v1/messages`) — отдельная capability для Claude Code; не считать OpenAI Chat Completions достаточным.
API gateway может проксировать запросы в llama-server или LM Studio, но должен корректно транслировать протоколы, streaming, ошибки, отмену и tool calls. Нельзя возвращать фиктивный успех.

## 7. Модельные capabilities
Для каждой модели хранить/получать:
- model id и display name;
- путь и SHA-256 файла;
- размер;
- backend;
- контекстный лимит (из метаданных/конфигурации, с отметкой confidence);
- поддержка chat, streaming, tools/function calling, vision, embeddings;
- фактическая проверка tool calling;
- заметки о совместимости.
Нельзя автоматически включать tools, если модель/runtime не прошли тест tool-call roundtrip.

## 8. Состояния
Runtime: Stopped → Starting → Loading → Ready → Stopping → Stopped; Failed — с диагностикой.
Integration: NotDetected / Detected / Configurable / Applying / Configured / Verified / Failed / ManualActionRequired.
Любое состояние должно быть основано на проверяемом факте.

## 9. Нефункциональные требования
- UI не блокируется во время загрузки модели.
- Все долгие операции асинхронны, отменяемы, с timeout.
- Все процессы имеют PID, stdout/stderr log capture, graceful shutdown и принудительное завершение как fallback.
- Никаких shell command strings из пользовательского ввода.
- Логи с ротацией; секреты и содержимое prompt по умолчанию не логировать.
- Все изменения сторонних конфигов атомарны и обратимы.
- Приложение не должно терять пользовательские настройки при обновлении.
- Ошибки содержат причину, диагностический код и рекомендуемое действие.

## 10. Критерии готовности
Релиз не допускается, пока не пройдены все P0 тесты в TEST_PLAN.md, включая cold start, API smoke tests, integration config round-trip, rollback, crash recovery и чистую установку на Windows 10/11.


## 11. Versioned integration policy
Integration adapters must be version-aware. Each adapter records detected product version, documented setup URL, config schema/version, protocol, and last successful verification. A documentation example is not proof of support for every release. On unknown versions, use safe manual guidance rather than writing config. The compatibility matrix is a release artifact.
