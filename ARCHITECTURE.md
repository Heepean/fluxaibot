# Архитектура

## Слои
1. **Tauri UI** — только представление и вызов типизированных команд.
2. **Application layer** — сценарии StartModel, StopModel, DiscoverApps, ConnectIntegration, RollbackIntegration.
3. **Domain layer** — Model, RuntimeProfile, RuntimeState, IntegrationStatus, Capability, ConfigTransaction.
4. **Infrastructure** — process manager, filesystem, SQLite, HTTP client/server, Windows registry/app discovery, logging.
5. **Runtime adapters** — llama.cpp, LM Studio; далее другие провайдеры.
6. **Integration adapters** — Claude Code, Codex, VS Code, OpenCode, LM Studio, Bionic.

## Предлагаемая структура crate
- `src-tauri/src/main.rs`
- `src-tauri/src/app_state.rs`
- `src-tauri/src/commands/`
- `src-tauri/src/domain/`
- `src-tauri/src/application/`
- `src-tauri/src/runtime/`
- `src-tauri/src/gateway/`
- `src-tauri/src/integrations/`
- `src-tauri/src/config_tx/`
- `src-tauri/src/platform/windows/`
- `src-tauri/src/storage/`
- `src-tauri/src/diagnostics/`
- `src-tauri/src/security/`
- `src-tauri/migrations/`
- `ui/src/`

## Главные интерфейсы
`RuntimeAdapter`: discover(), capabilities(), start(profile), wait_ready(timeout), list_models(), stop(), diagnostics().
`IntegrationAdapter`: detect(), inspect_config(), plan_changes(endpoint, model), apply(transaction), verify(), rollback(transaction).
`ConfigStore`: snapshot(path), atomic_write(path, bytes), restore(snapshot), validate(path).
`ProcessSupervisor`: spawn(executable, args, env_allowlist, working_dir), wait(), terminate_gracefully(), kill(), collect_logs().

## Принципы
- Ports-and-adapters; не связывать UI напрямую с llama.cpp или конкретным приложением.
- Ошибки типизированы через `thiserror`; контекст через `anyhow` только на границах.
- `tokio` для async runtime; не использовать блокирующий std::process в UI thread.
- `reqwest` для HTTP client; `axum` для локального API, если gateway является собственным процессом.
- `serde`/`serde_json`; TOML parser для Codex config.
- SQLite через `sqlx` или `rusqlite`; выбрать один, не смешивать без причины.
- Все версии runtime и integration adapters должны быть явно записаны в diagnostics.

## Поток запуска
UI command → application use case → validate profile → reserve port → spawn runtime → readiness poll → query models → API smoke request → state Ready.
При любой ошибке: stop child process → collect logs → release port reservation → state Failed with diagnostic.

## Изоляция
В первом релизе UI и service могут быть одним Tauri process, но HTTP server/runtime должны иметь независимый жизненный цикл. Рассмотреть отдельный service process только если доказана необходимость; не усложнять MVP.
