# Runtime Engine

## MVP backend
`llama.cpp` `llama-server` (GGUF). Версия фиксируется в release manifest. Бинарник и библиотеки должны поставляться из доверенного официального release/build pipeline; записывать URL источника, версию, лицензию и SHA-256.

## Запуск
- Аргументы формировать как массив `OsString`; запрещён `cmd.exe /c` и PowerShell для запуска runtime.
- Путь модели проверять canonicalize + existence + regular file.
- Порт назначать через bind probe; учитывать race condition — после spawn проверить реальный bind и health.
- Логи stdout/stderr читать постоянно, чтобы процесс не завис на заполненных pipe.
- Runtime child PID сохранять в сессии; при рестарте проверять, что PID принадлежит ожидаемому executable/path.
- Graceful shutdown; затем timeout; затем kill.
- Не убивать чужой процесс, даже если он слушает выбранный порт.

## Параметры
Профиль включает model_path, context_size, threads, batch_size, gpu_layers/offload, mmap/mlock policy, port, host, runtime_args_allowlist.
Не показывать опасные/неизвестные runtime args в простом режиме. Advanced args должны быть явными и валидироваться.

## GPU
- CPU backend — обязательный fallback.
- Vulkan — предпочтительный общий путь для AMD/Intel/поддерживаемых NVIDIA GPU, если конкретный бинарник собран с Vulkan.
- Проверять capabilities конкретного runtime binary; не выводить GPU availability только по названию видеокарты.
- GPU offload не равен полной загрузке модели в VRAM; учитывать KV cache и контекст.
- Показать предупреждение об оценочном характере memory estimate.
- Если GPU init fails — не делать бесконечный цикл; предложить CPU fallback после согласия.

## Модели
GGUF only в MVP. Сканирование каталогов ограничить пользовательскими выбранными roots. Не следовать symlinks/reparse points за пределы root без явного разрешения. SHA-256 считать потоково.
