const TauriAPI = {
    async invoke(command, args = {}) {
        const invoke = window.__TAURI__?.core?.invoke;
        if (!invoke) {
            throw new Error('Команды доступны только в настольном приложении FluxAiBot.');
        }
        return invoke(command, args);
    }
};

const app = {
    currentSection: 'dashboard',
    modelsFolder: '',
    models: [],
    status: null,
    statusRequest: null,
    messageTimer: null,
    heartbeat: null,
    hubModels: [],
    hubDownloadPoll: null,

    async init() {
        window.FluxAiBotI18n.init();
        this.setupNavigation();
        this.setupActions();
        this.setupModelSearch();
        this.applyTheme(this.getSavedTheme());
        this.addLog('Интерфейс FluxAiBot запущен.');
        await Promise.all([this.loadStatus(), this.loadIntegrations()]);
        this.startHeartbeat();
    },

    setupNavigation() {
        document.querySelectorAll('.nav-item[data-section]').forEach(button => {
            button.addEventListener('click', () => this.navigateTo(button.dataset.section));
        });
    },

    setupActions() {
        document.addEventListener('click', event => {
            const button = event.target.closest('[data-action]');
            if (!button) return;
            if (button.tagName === 'A') event.preventDefault();
            this.handleAction(button.dataset.action, button);
        });

        document.getElementById('theme-select').addEventListener('change', event => {
            this.applyTheme(event.target.value);
        });
        document.getElementById('language-select').addEventListener('change', event => {
            try {
                window.FluxAiBotI18n.setLanguage(event.target.value);
            } catch (error) {
                this.showMessage(`Не удалось сохранить язык интерфейса: ${this.errorMessage(error)}`);
            }
        });
        document.getElementById('hub-search').addEventListener('keydown', event => {
            if (event.key === 'Enter') this.searchHubModels();
        });
        document.getElementById('hub-recommended-only').addEventListener('change', () => {
            this.renderHubModels();
        });
        document.getElementById('hub-purpose-filter').addEventListener('change', () => {
            this.renderHubModels();
        });
    },

    setupModelSearch() {
        document.getElementById('model-search').addEventListener('input', () => {
            this.renderModels(this.models);
        });
    },

    handleAction(action, button) {
        const actions = {
            'go-dashboard': () => this.navigateTo('dashboard'),
            'go-models': () => this.navigateTo('models'),
            'go-gateway': () => this.navigateTo('gateway'),
            'refresh': () => this.refresh(),
            'refresh-integrations': () => this.loadIntegrations(),
            'select-folder': () => this.selectModelsFolder(),
            'scan-models': () => this.scanModels(),
            'search-hub-models': () => this.searchHubModels(),
            'download-hub-model': () => this.downloadHubModel(button),
            'cancel-hub-download': () => this.cancelHubDownload(),
            'start-model': () => this.startModel(button.dataset.modelId, button),
            'stop-runtime': () => this.stopRuntime(),
            'copy-url': () => this.copyUrl(),
            'toggle-token': () => this.toggleToken(),
            'copy-token': () => this.copyToken(),
            'save-settings': () => this.saveSettings(),
            'collect-diagnostics': () => this.collectDiagnostics(),
            'clear-log': () => this.clearLog()
        };
        const handler = actions[action];
        if (handler) handler();
    },

    navigateTo(section) {
        const target = document.getElementById(section);
        if (!target || !target.classList.contains('section')) return;
        if (section !== 'gateway') this.hideToken();
        this.currentSection = section;

        document.querySelectorAll('.nav-item[data-section]').forEach(button => {
            const active = button.dataset.section === section;
            button.classList.toggle('is-active', active);
            if (active) button.setAttribute('aria-current', 'page');
            else button.removeAttribute('aria-current');
        });
        document.querySelectorAll('.section').forEach(page => {
            const active = page === target;
            page.hidden = !active;
            page.classList.toggle('is-active', active);
        });
        document.getElementById('page-title').textContent = target.dataset.pageTitle;
        document.querySelector('.main-content').scrollTop = 0;
    },

    async refresh() {
        await Promise.all([this.loadStatus(), this.loadIntegrations()]);
        this.addLog('Состояние приложения обновлено.');
    },

    showMessage(message, type = 'error') {
        const element = document.getElementById('app-message');
        window.clearTimeout(this.messageTimer);
        element.textContent = message || '';
        element.className = message ? `app-message ${type}` : 'app-message';
        element.hidden = !message;
        if (message && type === 'success') {
            this.messageTimer = window.setTimeout(() => {
                element.hidden = true;
            }, 4200);
        }
    },

    async loadStatus() {
        if (this.statusRequest) return this.statusRequest;
        this.statusRequest = (async () => {
            try {
                const data = await TauriAPI.invoke('get_status');
                this.status = data;
                this.updateStatus(data);
                return data;
            } catch (error) {
                this.showMessage(`Не удалось получить состояние приложения: ${this.errorMessage(error)}`);
                this.updateConnectionStatus(false, 'Недоступен');
                return null;
            } finally {
                this.statusRequest = null;
            }
        })();
        return this.statusRequest;
    },

    updateStatus(data) {
        const runtimeState = String(data.runtime_state || 'stopped').toLowerCase();
        const labels = {
            ready: 'Запущена',
            starting: 'Запуск…',
            loading: 'Загрузка…',
            stopping: 'Остановка…',
            failed: 'Ошибка',
            stopped: 'Остановлена'
        };
        const hints = {
            ready: 'Принимает локальные запросы',
            starting: 'Подготовка runtime',
            loading: 'Загрузка весов модели',
            stopping: 'Завершение runtime',
            failed: 'Нужна проверка ошибки',
            stopped: 'Готова к запуску'
        };
        const activeModel = data.active_model?.display_name || 'Модель не выбрана';
        const apiUrl = data.api_url ? new URL(data.api_url) : null;
        const port = apiUrl?.port || '—';
        const gatewayRunning = Boolean(data.gateway_running);
        const stateLabel = labels[runtimeState] || runtimeState;

        document.getElementById('stat-runtime').textContent = stateLabel;
        document.getElementById('runtime-state-hint').textContent = hints[runtimeState] || 'Состояние обновлено';
        document.getElementById('stat-models').textContent = String(data.model_count ?? this.models.length);
        document.getElementById('nav-model-count').textContent = String(data.model_count ?? this.models.length);
        document.getElementById('stat-port').textContent = port;
        document.getElementById('gateway-port').textContent = port;
        document.getElementById('active-model').textContent = activeModel;
        document.getElementById('data-dir').textContent = data.data_dir || 'Недоступно';
        document.getElementById('folder-path').textContent = data.models_folder || this.modelsFolder || 'Не выбрана';
        document.getElementById('gateway-url').textContent = data.api_url || 'Недоступно';
        document.getElementById('gateway-url-input').value = data.api_url || '';
        document.getElementById('runtime-error').textContent = data.runtime_error || '';
        document.getElementById('runtime-warning').textContent = data.runtime_warning || '';
        document.getElementById('gateway-error').textContent = data.gateway_error || '';
        document.getElementById('runtime-summary').textContent = this.runtimeSummary(
            runtimeState,
            activeModel,
            data.runtime_backend,
            data.compute_device
        );
        this.modelsFolder = data.models_folder || this.modelsFolder;

        this.updateRuntimeIndicator(runtimeState);
        this.updateConnectionStatus(gatewayRunning, gatewayRunning ? 'Работает' : 'Недоступен');
        this.updateRuntimeControls(runtimeState);
        if (this.models.length) this.renderModels(this.models);
    },

    runtimeSummary(state, modelName, backend, device) {
        if (state === 'ready') {
            const accelerator = backend
                ? ` Режим: ${backend.toUpperCase()}${device?.name ? ` · ${device.name}` : ''}.`
                : '';
            return `${modelName} запущена и готова принимать локальные запросы.${accelerator}`;
        }
        if (state === 'starting' || state === 'loading') return 'Подождите, пока runtime подготовит модель.';
        if (state === 'stopping') return 'Runtime завершает работу.';
        if (state === 'failed') return 'Не удалось запустить модель. Подробности указаны в сообщении об ошибке.';
        return 'Выберите GGUF-файл и запустите его, чтобы начать работу.';
    },

    updateRuntimeIndicator(state) {
        const dot = document.getElementById('status-dot');
        const label = document.getElementById('status-text');
        const status = state === 'ready'
            ? 'online'
            : ['starting', 'loading', 'stopping'].includes(state)
                ? 'loading'
                : state === 'failed' ? 'error' : 'offline';
        const labels = {
            online: 'Модель активна',
            loading: 'Модель запускается',
            offline: 'Модель остановлена',
            error: 'Ошибка runtime'
        };
        dot.className = `status-dot ${status}`;
        label.textContent = labels[status];
    },

    updateConnectionStatus(connected, label) {
        const status = connected ? 'online' : 'offline';
        const dot = document.getElementById('gateway-status-dot');
        dot.className = `status-dot ${status}`;
        document.getElementById('gateway-status-short').textContent = connected ? 'API доступен' : 'API недоступен';
        document.getElementById('gateway-status').textContent = label;
        document.getElementById('gateway-status').className = `mini-status ${connected ? 'online' : 'status-stopped'}`;
        document.getElementById('gateway-status-card').textContent = label;
        const detail = document.getElementById('gateway-status-detail');
        detail.classList.toggle('is-connected', connected);
        detail.querySelector('.status-dot').className = `status-dot ${status}`;
        detail.lastElementChild.textContent = label;
    },

    updateRuntimeControls(state) {
        const canStop = ['ready', 'starting', 'loading'].includes(state);
        document.getElementById('btn-stop-dashboard').disabled = !canStop;
    },

    async loadIntegrations() {
        const button = document.querySelector('[data-action="refresh-integrations"]');
        if (button) button.disabled = true;
        try {
            const integrations = await TauriAPI.invoke('discover_integrations');
            if (!Array.isArray(integrations)) {
                throw new Error('Приложение вернуло некорректный список интеграций.');
            }
            const found = integrations.filter(integration =>
                ['Detected', 'Configured', 'Verified'].includes(integration.status)
            ).length;
            document.getElementById('stat-integrations').textContent = String(found);
            this.renderIntegrations(integrations);
        } catch (error) {
            this.showMessage(`Не удалось проверить приложения: ${this.errorMessage(error)}`);
            document.getElementById('integrations-grid').replaceChildren(
                this.createEmptyState('Список приложений недоступен', this.errorMessage(error))
            );
        } finally {
            if (button) button.disabled = false;
        }
    },

    renderIntegrations(integrations) {
        const grid = document.getElementById('integrations-grid');
        grid.replaceChildren();
        if (!integrations.length) {
            grid.append(this.createEmptyState('Приложения не найдены', 'Список интеграций пуст.'));
            return;
        }

        integrations.forEach(integration => {
            const card = document.createElement('article');
            card.className = 'integration-card';

            const header = document.createElement('div');
            header.className = 'integration-header';
            const name = document.createElement('h2');
            name.className = 'integration-name';
            name.textContent = integration.app || 'Неизвестное приложение';

            const rawStatus = String(integration.status || 'Unknown');
            const statusClass = rawStatus
                .replace(/([a-z])([A-Z])/g, '$1-$2')
                .replace(/_/g, '-')
                .toLowerCase();
            const status = document.createElement('span');
            status.className = `integration-status status-${statusClass}`;
            status.textContent = this.integrationStatusLabel(rawStatus);
            header.append(name, status);
            card.append(header);

            if (integration.detected_version) {
                const version = document.createElement('p');
                version.className = 'integration-version';
                version.textContent = `Обнаружена версия ${integration.detected_version}`;
                card.append(version);
            }
            if (integration.notes) {
                const notes = document.createElement('p');
                notes.className = 'integration-notes';
                notes.textContent = integration.notes;
                card.append(notes);
            }

            const features = document.createElement('div');
            features.className = 'integration-features';
            (integration.features || []).forEach(feature => {
                const row = document.createElement('div');
                row.className = 'feature-item';
                const mark = document.createElement('span');
                mark.className = `feature-mark${feature.supported ? ' is-supported' : ''}`;
                mark.textContent = feature.supported ? '✓' : '—';
                const text = document.createElement('span');
                text.textContent = feature.note
                    ? `${feature.name}: ${feature.note}`
                    : feature.name;
                row.append(mark, text);
                features.append(row);
            });
            card.append(features);
            grid.append(card);
        });
    },

    integrationStatusLabel(status) {
        const labels = {
            NotDetected: 'Не найдено',
            Detected: 'Найдено',
            Configurable: 'Требует настройки',
            Applying: 'Настройка',
            Configured: 'Настроено',
            Verified: 'Проверено',
            Failed: 'Ошибка проверки',
            ManualActionRequired: 'Нужно действие',
            Unsupported: 'Не поддерживается'
        };
        return labels[status] || 'Статус неизвестен';
    },

    startHeartbeat() {
        if (this.heartbeat) window.clearInterval(this.heartbeat);
        this.heartbeat = window.setInterval(() => {
            if (document.visibilityState === 'visible') this.loadStatus();
        }, 5000);
    },

    addLog(message) {
        const log = document.getElementById('activity-log');
        const empty = log.querySelector('.log-empty');
        if (empty) empty.remove();

        const entry = document.createElement('div');
        entry.className = 'log-entry';
        const time = document.createElement('time');
        time.className = 'log-time';
        time.dateTime = new Date().toISOString();
        time.textContent = new Date().toLocaleTimeString();
        const text = document.createElement('span');
        text.className = 'log-msg';
        text.textContent = message;
        entry.append(time, text);
        log.append(entry);

        while (log.children.length > 60) log.firstElementChild.remove();
        log.scrollTop = log.scrollHeight;
    },

    clearLog() {
        const log = document.getElementById('activity-log');
        log.replaceChildren();
        const empty = document.createElement('div');
        empty.className = 'log-empty';
        empty.textContent = 'Журнал очищен. Новые события появятся здесь.';
        log.append(empty);
    },

    async selectModelsFolder() {
        try {
            const folder = await TauriAPI.invoke('open_folder_dialog');
            if (!folder) return;
            this.modelsFolder = folder;
            document.getElementById('folder-path').textContent = folder;
            this.showMessage('');
            await this.scanModels();
        } catch (error) {
            this.showMessage(`Не удалось выбрать папку: ${this.errorMessage(error)}`);
        }
    },

    async scanModels() {
        if (!this.modelsFolder) {
            this.showMessage('Сначала выберите папку с моделями.');
            return;
        }
        const button = document.getElementById('btn-scan');
        button.disabled = true;
        button.innerHTML = '<span aria-hidden="true">↻</span> Сканирование…';
        this.showMessage('');
        try {
            const result = await TauriAPI.invoke('scan_models', { folders: [this.modelsFolder] });
            if (!Array.isArray(result?.models)) {
                throw new Error('Приложение вернуло некорректный список моделей.');
            }
            this.models = result.models;
            this.renderModels(this.models);
            document.getElementById('stat-models').textContent = String(this.models.length);
            document.getElementById('nav-model-count').textContent = String(this.models.length);
            this.addLog(`Сканирование завершено: найдено GGUF-моделей — ${this.models.length}.`);
            this.showMessage(`Сканирование завершено. Найдено моделей: ${this.models.length}.`, 'success');
        } catch (error) {
            this.showMessage(`Ошибка сканирования моделей: ${this.errorMessage(error)}`);
        } finally {
            button.disabled = false;
            button.innerHTML = '<span aria-hidden="true">↻</span> Сканировать';
        }
    },

    async searchHubModels() {
        const button = document.getElementById('hub-search-button');
        const query = document.getElementById('hub-search').value.trim();
        const maxSize = document.getElementById('hub-size-filter').value;
        button.disabled = true;
        button.textContent = 'Ищем…';
        document.getElementById('hub-results-count').textContent = 'Запрос к Hugging Face…';
        try {
            const result = await TauriAPI.invoke('search_huggingface_models', {
                query,
                maxSizeGb: maxSize ? Number(maxSize) : null
            });
            if (!Array.isArray(result?.models)) {
                throw new Error('Hugging Face вернул некорректный список моделей.');
            }
            this.hubModels = result.models;
            const devices = Array.isArray(result.compute_devices) ? result.compute_devices : [];
            const gpuInfo = result.available_vram_bytes
                ? ` · VRAM: ${this.formatBytes(result.available_vram_bytes)} свободно`
                : '';
            const deviceInfo = devices.length ? ` · ${devices.map(device => device.name).join(', ')}` : '';
            document.getElementById('hub-hardware-info').textContent =
                `ОЗУ: ${this.formatBytes(result.total_memory_bytes)} всего, ${this.formatBytes(result.available_memory_bytes)} доступно сейчас${gpuInfo}${deviceInfo}`;
            this.renderHubModels();
            const warnings = [];
            if (result.warnings?.length) {
                warnings.push(`Часть результатов Hugging Face пропущена. Например: ${result.warnings[0]}`);
            }
            if (result.hardware_warnings?.length) {
                warnings.push(`Проверка GPU: ${result.hardware_warnings[0]}`);
            }
            if (warnings.length) {
                this.showMessage(warnings.join(' '));
            }
        } catch (error) {
            document.getElementById('hub-results-count').textContent = 'Не удалось получить каталог';
            document.getElementById('hub-models-list').replaceChildren(
                this.createEmptyState('Поиск не выполнен', this.errorMessage(error))
            );
            this.showMessage(`Ошибка поиска на Hugging Face: ${this.errorMessage(error)}`);
        } finally {
            button.disabled = false;
            button.textContent = 'Найти модели';
        }
    },

    renderHubModels() {
        const grid = document.getElementById('hub-models-list');
        const recommendedOnly = document.getElementById('hub-recommended-only').checked;
        const purpose = document.getElementById('hub-purpose-filter').value;
        const models = this.hubModels.filter(model =>
            (!recommendedOnly || model.recommended_for_memory)
            && (purpose === 'all' || model.purpose === purpose)
        );
        document.getElementById('hub-results-count').textContent =
            `${models.length} моделей${purpose === 'coding' ? ' для кодинга' : purpose === 'universal' ? ' универсальных' : ''}${recommendedOnly ? ' подходят по текущей доступной памяти' : ''}`;
        grid.replaceChildren();
        if (!models.length) {
            grid.append(this.createEmptyState(
                this.hubModels.length ? 'Нет моделей под выбранный фильтр' : 'Модели не найдены',
                this.hubModels.length
                    ? 'Снимите фильтр по задаче или памяти, либо увеличьте максимальный размер.'
                    : 'Измените запрос и попробуйте снова.'
            ));
            return;
        }

        models.forEach(model => {
            const card = document.createElement('article');
            card.className = 'model-card hub-model-card';

            const head = document.createElement('div');
            head.className = 'model-card-head';
            const symbol = document.createElement('span');
            symbol.className = 'model-symbol';
            symbol.textContent = 'HF';
            const title = document.createElement('div');
            title.className = 'model-title-wrap';
            const name = document.createElement('h3');
            name.className = 'model-name';
            name.textContent = model.repo_id;
            const meta = document.createElement('span');
            meta.className = 'model-size';
            const locale = window.FluxAiBotI18n.getLocale();
            meta.textContent = `↓ ${Number(model.downloads || 0).toLocaleString(locale)} · ♥ ${Number(model.likes || 0).toLocaleString(locale)}`;
            title.append(name, meta);
            head.append(symbol, title);
            const repoLink = document.createElement('a');
            repoLink.className = 'hub-repo-link';
            repoLink.href = `https://huggingface.co/${model.repo_id.split('/').map(encodeURIComponent).join('/')}`;
            repoLink.target = '_blank';
            repoLink.rel = 'noopener noreferrer';
            repoLink.textContent = 'Карточка ↗';
            head.append(repoLink);

            const status = document.createElement('span');
            status.className = `model-badge${model.recommended_for_memory ? '' : ' model-badge-warning'}`;
            status.textContent = model.recommended_for_memory ? 'Есть вариант по ОЗУ/VRAM*' : 'Может не поместиться';
            head.append(status);
            const purposeBadge = document.createElement('span');
            purposeBadge.className = 'model-badge';
            purposeBadge.textContent = model.purpose === 'coding' ? 'Для кодинга*' : 'Универсальная';
            head.append(purposeBadge);

            const description = document.createElement('p');
            description.className = 'hub-model-description';
            description.textContent = `Лицензия: ${model.license || 'не указана в карточке'} · ориентир памяти от ${this.formatBytes(model.estimated_ram_bytes)} с учётом доступных ОЗУ и VRAM. Это оценка; сверяйте требования на странице автора.`;

            const controls = document.createElement('div');
            controls.className = 'hub-model-controls';
            const variant = document.createElement('select');
            variant.className = 'select-control';
            variant.setAttribute('aria-label', `Вариант GGUF для ${model.repo_id}`);
            model.variants.forEach((item, index) => {
                const option = document.createElement('option');
                option.value = String(index);
                option.textContent = `${item.label} · ${this.formatBytes(item.size_bytes)} · ОЗУ ~${this.formatBytes(item.estimated_ram_bytes)}${item.recommended_for_memory ? ' · подходит' : ''}${item.files.length > 1 ? ` · ${item.files.length} частей` : ''}`;
                variant.append(option);
            });
            const download = document.createElement('button');
            download.type = 'button';
            download.className = 'btn btn-primary btn-small';
            download.dataset.action = 'download-hub-model';
            download.dataset.repoId = model.repo_id;
            download.dataset.revision = model.revision;
            download.dataset.modelIndex = String(this.hubModels.indexOf(model));
            download.textContent = 'Скачать';
            controls.append(variant, download);

            card.append(head, description, controls);
            grid.append(card);
        });
    },

    async downloadHubModel(button) {
        const modelIndex = Number(button.dataset.modelIndex);
        const model = this.hubModels[modelIndex];
        const select = button.parentElement.querySelector('select');
        const selected = model?.variants[Number(select?.value || 0)];
        if (!model || !selected) {
            this.showMessage('Не удалось определить вариант модели для скачивания.');
            return;
        }
        button.disabled = true;
        button.textContent = 'Подготовка…';
        this.showHubDownloadProgress({
            state: 'preparing',
            repo_id: model.repo_id,
            downloaded_bytes: 0,
            total_bytes: selected.size_bytes
        });
        try {
            await TauriAPI.invoke('start_huggingface_download', {
                repoId: model.repo_id,
                revision: model.revision,
                files: selected.files
            });
            this.pollHubDownload();
        } catch (error) {
            button.disabled = false;
            button.textContent = 'Скачать';
            document.getElementById('hub-download-progress').hidden = true;
            this.showMessage(`Не удалось начать загрузку: ${this.errorMessage(error)}`);
        }
    },

    pollHubDownload() {
        if (this.hubDownloadPoll) window.clearInterval(this.hubDownloadPoll);
        const update = async () => {
            try {
                const progress = await TauriAPI.invoke('get_huggingface_download_progress');
                this.showHubDownloadProgress(progress);
                if (['completed', 'cancelled', 'failed'].includes(progress.state)) {
                    window.clearInterval(this.hubDownloadPoll);
                    this.hubDownloadPoll = null;
                    if (progress.state === 'completed') {
                        this.showMessage('Модель загружена и добавлена в библиотеку.', 'success');
                        await this.loadStatus();
                        if (this.modelsFolder) await this.scanModels();
                    } else if (progress.state === 'failed') {
                        this.showMessage(`Ошибка загрузки модели: ${progress.error || 'неизвестная ошибка'}`);
                    } else {
                        this.showMessage('Загрузка модели отменена.', 'success');
                    }
                    document.querySelectorAll('[data-action="download-hub-model"]').forEach(button => {
                        button.disabled = false;
                        button.textContent = 'Скачать';
                    });
                }
            } catch (error) {
                window.clearInterval(this.hubDownloadPoll);
                this.hubDownloadPoll = null;
                this.showMessage(`Не удалось получить состояние загрузки: ${this.errorMessage(error)}`);
            }
        };
        update();
        this.hubDownloadPoll = window.setInterval(update, 800);
    },

    showHubDownloadProgress(progress) {
        const panel = document.getElementById('hub-download-progress');
        panel.hidden = false;
        const stateLabels = {
            preparing: 'Подготовка загрузки…',
            downloading: `Загрузка ${progress.file_name || ''}`.trim(),
            verifying: 'Проверка и сохранение модели…',
            completed: 'Загрузка завершена',
            cancelled: 'Загрузка отменена',
            failed: 'Ошибка загрузки'
        };
        const downloaded = Number(progress.downloaded_bytes || 0);
        const total = Number(progress.total_bytes || 0);
        const percentage = total > 0 ? Math.min(100, Math.floor(downloaded / total * 100)) : 0;
        document.getElementById('hub-progress-title').textContent =
            stateLabels[progress.state] || 'Подготовка загрузки…';
        document.getElementById('hub-progress-value').textContent =
            `${percentage}%`;
        document.getElementById('hub-progress-bar').value = percentage;
        document.getElementById('hub-progress-details').textContent =
            `${this.formatBytes(downloaded)} из ${this.formatBytes(total)} · ${progress.repo_id || ''}`;
        document.getElementById('hub-cancel-download').hidden =
            ['completed', 'cancelled', 'failed', 'idle'].includes(progress.state);
    },

    async cancelHubDownload() {
        const button = document.getElementById('hub-cancel-download');
        button.disabled = true;
        try {
            await TauriAPI.invoke('cancel_huggingface_download');
        } catch (error) {
            button.disabled = false;
            this.showMessage(`Не удалось отменить загрузку: ${this.errorMessage(error)}`);
        }
    },

    renderModels(modelList) {
        const grid = document.getElementById('models-list');
        const locale = window.FluxAiBotI18n.getLocale();
        const query = document.getElementById('model-search').value.trim().toLocaleLowerCase(locale);
        const filtered = modelList.filter(model =>
            String(model.display_name || '').toLocaleLowerCase(locale).includes(query)
        );
        document.getElementById('model-count').textContent =
            query ? `${filtered.length} из ${modelList.length}` : String(modelList.length);
        grid.replaceChildren();

        if (!modelList.length) {
            grid.append(this.createEmptyState(
                'Моделей пока нет',
                'Выберите папку с файлами .gguf и нажмите «Сканировать».',
                'select-folder',
                'Выбрать папку'
            ));
            return;
        }
        if (!filtered.length) {
            grid.append(this.createEmptyState('Ничего не найдено', 'Измените поисковый запрос.'));
            return;
        }

        filtered.forEach(model => {
            const active = this.status?.active_model?.id === model.id;
            const projector = model.role === 'projector' || /mmproj/i.test(
                `${model.display_name || ''} ${model.path || ''}`
            );
            const card = document.createElement('article');
            card.className = `model-card${active ? ' is-active' : ''}${projector ? ' is-projector' : ''}`;

            const head = document.createElement('div');
            head.className = 'model-card-head';
            const symbol = document.createElement('span');
            symbol.className = 'model-symbol';
            symbol.textContent = 'AI';
            const title = document.createElement('div');
            title.className = 'model-title-wrap';
            const name = document.createElement('h3');
            name.className = 'model-name';
            name.textContent = model.display_name || 'Модель без названия';
            const size = document.createElement('span');
            size.className = 'model-size';
            size.textContent = this.formatBytes(model.size);
            title.append(name, size);
            head.append(symbol, title);
            if (projector) {
                const badge = document.createElement('span');
                badge.className = 'model-badge model-badge-projector';
                badge.textContent = 'Проектор';
                head.append(badge);
            } else if (active) {
                const badge = document.createElement('span');
                badge.className = 'model-badge';
                badge.textContent = 'Активна';
                head.append(badge);
            }

            const path = document.createElement('p');
            path.className = 'model-path';
            path.title = model.path || '';
            path.textContent = model.path || 'Путь к файлу недоступен';
            const note = document.createElement('p');
            note.className = 'model-note';
            note.textContent = projector
                ? 'Файл mmproj дополняет мультимодальную модель и не запускается отдельно. Выберите основной GGUF-файл.'
                : '';
            const footer = document.createElement('div');
            footer.className = 'model-card-footer';
            const format = document.createElement('span');
            format.className = 'subtle-text';
            format.textContent = 'GGUF';
            const start = document.createElement('button');
            start.type = 'button';
            start.className = `btn btn-small ${active ? 'btn-secondary' : 'btn-primary'}`;
            start.dataset.action = 'start-model';
            start.dataset.modelId = model.id;
            const runtimeState = this.status?.runtime_state;
            start.textContent = projector
                ? 'Не запускается отдельно'
                : active
                    ? 'Активна'
                    : runtimeState === 'ready'
                        ? 'Остановите текущую модель'
                        : 'Запустить';
            start.disabled = projector || active
                || ['ready', 'starting', 'loading', 'stopping'].includes(runtimeState);
            footer.append(format, start);
            card.append(head, path);
            if (projector) card.append(note);
            card.append(footer);
            grid.append(card);
        });
    },

    createEmptyState(title, description, action, actionLabel) {
        const state = document.createElement('div');
        state.className = 'empty-state';
        const icon = document.createElement('span');
        icon.className = 'empty-icon';
        icon.textContent = '◌';
        const heading = document.createElement('h3');
        heading.textContent = title;
        const text = document.createElement('p');
        text.textContent = description;
        state.append(icon, heading, text);
        if (action && actionLabel) {
            const button = document.createElement('button');
            button.type = 'button';
            button.className = 'btn btn-secondary';
            button.dataset.action = action;
            button.textContent = actionLabel;
            state.append(button);
        }
        return state;
    },

    async startModel(modelId, button) {
        if (!modelId) {
            this.showMessage('Не удалось определить выбранную модель. Просканируйте папку ещё раз.');
            return;
        }
        const model = this.models.find(item => item.id === modelId);
        if (!model) {
            this.showMessage('Модель больше не найдена в текущем списке. Просканируйте папку ещё раз.');
            return;
        }
        if (model.role === 'projector' || /mmproj/i.test(`${model.display_name || ''} ${model.path || ''}`)) {
            this.showMessage('Это файл визуального проектора mmproj, его нельзя запустить отдельно. Выберите основной GGUF-файл модели.');
            return;
        }
        if (button) {
            button.disabled = true;
            button.textContent = 'Запуск…';
        }
        this.showMessage('');
        try {
            const result = await TauriAPI.invoke('start_runtime', { modelId });
            this.addLog(result.already_running
                ? `Модель уже активна: ${result.model} (${result.runtime_version}).`
                : `Модель готова: ${result.model} (${result.runtime_version}).`);
            this.showMessage(
                result.already_running
                    ? `Модель «${model.display_name}» уже запущена.`
                    : `Модель «${model.display_name}» запущена.`,
                'success'
            );
        } catch (error) {
            this.showMessage(`Не удалось запустить модель: ${this.errorMessage(error)}`);
            this.addLog('Запуск модели завершился ошибкой.');
        } finally {
            await this.loadStatus();
        }
    },

    async stopRuntime() {
        const button = document.getElementById('btn-stop-dashboard');
        button.disabled = true;
        this.showMessage('');
        try {
            await TauriAPI.invoke('stop_runtime');
            this.addLog('Модель остановлена.');
            this.showMessage('Локальная модель остановлена.', 'success');
            await this.loadStatus();
        } catch (error) {
            this.showMessage(`Не удалось остановить модель: ${this.errorMessage(error)}`);
        } finally {
            const state = this.status?.runtime_state;
            button.disabled = !['ready', 'starting', 'loading'].includes(state);
        }
    },

    async copyUrl() {
        const url = document.getElementById('gateway-url-input').value;
        if (!url) {
            this.showMessage('Адрес API ещё не загружен.');
            return;
        }
        try {
            await navigator.clipboard.writeText(url);
            this.showMessage('Адрес API скопирован.', 'success');
        } catch (error) {
            this.showMessage(`Не удалось скопировать адрес: ${this.errorMessage(error)}`);
        }
    },

    async toggleToken() {
        const input = document.getElementById('gateway-token');
        const button = document.getElementById('token-toggle');
        if (input.type === 'text') {
            this.hideToken();
            return;
        }
        try {
            input.value = await TauriAPI.invoke('get_api_token');
            input.type = 'text';
            button.textContent = 'Скрыть';
            button.setAttribute('aria-expanded', 'true');
        } catch (error) {
            this.showMessage(`Не удалось получить токен: ${this.errorMessage(error)}`);
        }
    },

    hideToken() {
        const input = document.getElementById('gateway-token');
        const button = document.getElementById('token-toggle');
        if (!input || !button) return;
        input.value = '';
        input.type = 'password';
        button.textContent = 'Показать';
        button.setAttribute('aria-expanded', 'false');
    },

    async copyToken() {
        try {
            const token = await TauriAPI.invoke('get_api_token');
            await navigator.clipboard.writeText(token);
            this.showMessage('Токен скопирован в буфер обмена.', 'success');
            this.addLog('Токен API скопирован в буфер обмена.');
        } catch (error) {
            this.showMessage(`Не удалось скопировать токен: ${this.errorMessage(error)}`);
        }
    },

    saveSettings() {
        const theme = document.getElementById('theme-select').value;
        try {
            localStorage.setItem('fluxaibot-theme', theme);
            this.applyTheme(theme);
            this.addLog('Тема оформления сохранена.');
            this.showMessage('Тема оформления сохранена на этом устройстве.', 'success');
        } catch (error) {
            this.showMessage(`Не удалось сохранить тему: ${this.errorMessage(error)}`);
        }
    },

    getSavedTheme() {
        return localStorage.getItem('fluxaibot-theme') || 'dark';
    },

    applyTheme(theme) {
        const selected = theme === 'light' ? 'light' : 'dark';
        document.documentElement.setAttribute('data-theme', selected);
        const select = document.getElementById('theme-select');
        if (select) select.value = selected;
    },

    async collectDiagnostics() {
        const output = document.getElementById('diagnostics-output');
        const button = document.querySelector('[data-action="collect-diagnostics"]');
        button.disabled = true;
        try {
            const status = await TauriAPI.invoke('get_status');
            output.textContent = JSON.stringify(status, null, 2);
            this.showMessage('');
            this.addLog('Диагностическая сводка обновлена.');
        } catch (error) {
            this.showMessage(`Не удалось собрать диагностику: ${this.errorMessage(error)}`);
        } finally {
            button.disabled = false;
        }
    },

    formatBytes(bytes) {
        const value = Number(bytes);
        if (!Number.isFinite(value) || value < 0) return 'Размер неизвестен';
        const locale = window.FluxAiBotI18n.getLocale();
        const formatter = new Intl.NumberFormat(locale, {
            minimumFractionDigits: value < 1024 ** 3 ? 1 : 2,
            maximumFractionDigits: value < 1024 ** 3 ? 1 : 2
        });
        if (value < 1024 ** 3) return `${formatter.format(value / 1024 ** 2)} MB`;
        return `${formatter.format(value / 1024 ** 3)} GB`;
    },

    errorMessage(error) {
        return typeof error === 'string' ? error : error?.message || String(error);
    }
};

window.app = app;

document.addEventListener('DOMContentLoaded', () => {
    app.init().catch(error => {
        app.showMessage(`Не удалось загрузить интерфейс: ${app.errorMessage(error)}`);
    });
});
