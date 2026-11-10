# API Gateway

## Задача
Предоставлять стабильный localhost endpoint независимо от backend runtime. Gateway не должен выдавать capabilities, которых фактически нет.

## Безопасность
- Bind default `127.0.0.1`, не `0.0.0.0`.
- Случайный токен на установку/сессию; хранить защищённо через Windows Credential Manager/DPAPI, если требуется.
- Ограничить CORS; по умолчанию CORS off.
- Проверять Host header и Origin, защищаться от DNS rebinding.
- Не принимать произвольные URL для proxy targets.
- Rate limit и max request body; streaming timeout.
- Удалять секреты из логов.

## Протоколы
- OpenAI Chat Completions.
- OpenAI Responses API — отдельная реализация и тесты.
- Anthropic Messages API — отдельная реализация и тесты.
- SSE: корректные event framing, `[DONE]` где требует протокол, cancellation при закрытии клиента.
- Tool calls: сохранять id, type, function name, arguments и корректный finish_reason.
- Ошибки возвращать совместимым JSON error shape с HTTP status, не текстовую заглушку.

## Capabilities endpoint
`GET /hub/v1/capabilities` возвращает поддерживаемые protocols/endpoints и model capabilities. Клиенты интеграции могут быть подключены только если необходимый протокол присутствует.

## Обязательные тесты
Golden request/response tests, streaming parse tests, cancellation, malformed JSON, oversize body, tool call roundtrip, concurrent requests, runtime unavailable, token invalid.
