//! Application state shared across Tauri commands and the local API gateway.

use crate::domain::{CapabilityDocument, ModelRecord, RuntimeState};
use crate::integrations::IntegrationRegistry;
use crate::model_hub::{self, DownloadProgress, DownloadState, HubSearchResult};
use crate::runtime::gpu::{self, ComputeDevice, RuntimeBackend};
use crate::runtime::process_supervisor::ProcessSupervisor;
use crate::security::TokenManager;
use crate::storage::Storage;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};
use tokio::sync::{watch, Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio::time::Instant;

const RUNTIME_LABEL: &str = "llama-server";
const RUNTIME_PORT: u16 = 12346;
const GATEWAY_PORT: u16 = 12345;
const RUNTIME_READINESS_TIMEOUT: Duration = Duration::from_secs(300);
const RUNTIME_MANIFEST: &str = include_str!("../resources/runtime-manifest.json");
const EMBEDDED_CPU_RUNTIME_ID: u16 = 101;
const EMBEDDED_VULKAN_RUNTIME_ID: u16 = 102;
const EMBEDDED_CUDA_RUNTIME_ID: u16 = 103;

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(module_name: *const u16) -> *mut std::ffi::c_void;
    fn FindResourceW(
        module: *mut std::ffi::c_void,
        name: *const u16,
        resource_type: *const u16,
    ) -> *mut std::ffi::c_void;
    fn LoadResource(
        module: *mut std::ffi::c_void,
        resource: *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void;
    fn LockResource(resource: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn SizeofResource(module: *mut std::ffi::c_void, resource: *mut std::ffi::c_void) -> u32;
}

pub type AppResult<T> = Result<T, String>;

#[derive(Deserialize)]
struct RuntimeManifest {
    version: String,
    source_commit: String,
    sha256: String,
    #[serde(default)]
    vulkan_sha256: Option<String>,
    #[serde(default)]
    cuda_sha256: Option<String>,
    #[serde(default)]
    cuda_dependencies: Vec<RuntimeDependency>,
}

#[derive(Deserialize)]
struct RuntimeDependency {
    filename: String,
    sha256: String,
}

pub struct AppState {
    pub storage: Arc<Storage>,
    pub supervisor: Arc<ProcessSupervisor>,
    pub runtime_state: Arc<RwLock<RuntimeState>>,
    pub integrations: Arc<RwLock<IntegrationRegistry>>,
    pub capabilities: Arc<RwLock<CapabilityDocument>>,
    pub token: Arc<TokenManager>,
    pub gateway_port: u16,
    pub runtime_port: u16,
    pub gateway_running: Arc<RwLock<bool>>,
    pub gateway_error: Arc<RwLock<Option<String>>>,
    pub runtime_error: Arc<RwLock<Option<String>>>,
    pub runtime_warning: Arc<RwLock<Option<String>>>,
    pub runtime_backend: Arc<RwLock<Option<RuntimeBackend>>>,
    pub compute_device: Arc<RwLock<Option<ComputeDevice>>>,
    pub compute_devices: Arc<RwLock<Option<Vec<ComputeDevice>>>>,
    compute_device_warnings: Arc<RwLock<Option<Vec<String>>>>,
    pub active_model: Arc<RwLock<Option<ModelRecord>>>,
    pub models_folder: Arc<RwLock<Option<PathBuf>>>,
    pub model_roots: Arc<RwLock<Vec<PathBuf>>>,
    pub discovered_models: Arc<RwLock<Vec<ModelRecord>>>,
    pub http_client: reqwest::Client,
    pub download_progress: Arc<RwLock<DownloadProgress>>,
    operation_lock: Mutex<()>,
    hardware_probe_lock: Mutex<()>,
    runtime_cancel: Mutex<Option<watch::Sender<bool>>>,
    download_cancel: Mutex<Option<watch::Sender<bool>>>,
    download_task: Mutex<Option<JoinHandle<()>>>,
}

impl AppState {
    pub async fn new() -> Result<Arc<Self>, String> {
        let data_dir = crate::platform::Platform::app_data_dir()?;
        let storage = Arc::new(
            Storage::new(data_dir)
                .await
                .map_err(|e| format!("Failed to initialize storage: {e}"))?,
        );
        let models_folder = storage
            .get_setting("models_folder")
            .await
            .map_err(|e| format!("Failed to restore the last models folder: {e}"))?
            .map(PathBuf::from);
        let http_client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| format!("Failed to initialize HTTP client: {e}"))?;
        let capabilities = CapabilityDocument {
            protocols: vec!["openai-chat-completions".to_string()],
            endpoints: vec![
                "/health".to_string(),
                "/v1/models".to_string(),
                "/v1/chat/completions".to_string(),
                "/hub/v1/capabilities".to_string(),
                "/hub/v1/status".to_string(),
            ],
            model_capabilities: vec!["chat".to_string(), "streaming".to_string()],
        };

        Ok(Arc::new(Self {
            storage,
            supervisor: Arc::new(ProcessSupervisor::new()),
            runtime_state: Arc::new(RwLock::new(RuntimeState::Stopped)),
            integrations: Arc::new(RwLock::new(IntegrationRegistry::new())),
            capabilities: Arc::new(RwLock::new(capabilities)),
            token: Arc::new(TokenManager::generate()),
            gateway_port: GATEWAY_PORT,
            runtime_port: RUNTIME_PORT,
            gateway_running: Arc::new(RwLock::new(false)),
            gateway_error: Arc::new(RwLock::new(None)),
            runtime_error: Arc::new(RwLock::new(None)),
            runtime_warning: Arc::new(RwLock::new(None)),
            runtime_backend: Arc::new(RwLock::new(None)),
            compute_device: Arc::new(RwLock::new(None)),
            compute_devices: Arc::new(RwLock::new(None)),
            compute_device_warnings: Arc::new(RwLock::new(None)),
            active_model: Arc::new(RwLock::new(None)),
            models_folder: Arc::new(RwLock::new(models_folder)),
            model_roots: Arc::new(RwLock::new(Vec::new())),
            discovered_models: Arc::new(RwLock::new(Vec::new())),
            http_client,
            download_progress: Arc::new(RwLock::new(DownloadProgress::default())),
            operation_lock: Mutex::new(()),
            hardware_probe_lock: Mutex::new(()),
            runtime_cancel: Mutex::new(None),
            download_cancel: Mutex::new(None),
            download_task: Mutex::new(None),
        }))
    }

    pub async fn get_status(&self) -> AppResult<serde_json::Value> {
        let runtime_state = self.runtime_state.read().await.as_str().to_string();
        let models_folder = self.models_folder.read().await.clone();
        let active_model = self.active_model.read().await.clone();
        let model_count = self.discovered_models.read().await.len();
        let gateway_running = *self.gateway_running.read().await;
        let gateway_error = self.gateway_error.read().await.clone();
        let runtime_error = self.runtime_error.read().await.clone();
        let runtime_warning = self.runtime_warning.read().await.clone();
        let runtime_backend = self
            .runtime_backend
            .read()
            .await
            .map(|backend| backend.as_str());
        let compute_device = self.compute_device.read().await.clone();
        let compute_devices = self
            .compute_devices
            .read()
            .await
            .clone()
            .unwrap_or_default();

        Ok(json!({
            "runtime_state": runtime_state,
            "runtime_url": format!("http://127.0.0.1:{}", self.runtime_port),
            "api_url": format!("http://127.0.0.1:{}", self.gateway_port),
            "gateway_running": gateway_running,
            "gateway_error": gateway_error,
            "runtime_error": runtime_error,
            "runtime_warning": runtime_warning,
            "runtime_backend": runtime_backend,
            "compute_device": compute_device,
            "compute_devices": compute_devices,
            "data_dir": self.storage.data_dir().to_string_lossy().to_string(),
            "active_model": active_model.map(|model| json!({
                "id": model.id,
                "display_name": model.display_name,
            })),
            "model_count": model_count,
            "models_folder": models_folder.map(|path| path.to_string_lossy().to_string()),
        }))
    }

    pub async fn discover_integrations(&self) -> AppResult<serde_json::Value> {
        let registry = self.integrations.read().await;
        serde_json::to_value(registry.discover_all())
            .map_err(|e| format!("Failed to serialize integrations: {e}"))
    }

    pub async fn search_huggingface_models(
        &self,
        query: &str,
        max_size_gb: Option<u64>,
    ) -> AppResult<HubSearchResult> {
        let max_size_bytes = max_size_gb.map(|size| size.saturating_mul(1024 * 1024 * 1024));
        let (devices, hardware_warnings) = self.detect_compute_devices().await?;
        let mut result = model_hub::search_models(query, max_size_bytes, &devices).await?;
        result.hardware_warnings = hardware_warnings;
        Ok(result)
    }

    async fn detect_compute_devices(&self) -> AppResult<(Vec<ComputeDevice>, Vec<String>)> {
        let _probe = self.hardware_probe_lock.lock().await;
        if let Some(devices) = self.compute_devices.read().await.clone() {
            let warnings = self
                .compute_device_warnings
                .read()
                .await
                .clone()
                .unwrap_or_default();
            return Ok((devices, warnings));
        }

        let manifest: RuntimeManifest = serde_json::from_str(RUNTIME_MANIFEST)
            .map_err(|error| format!("Invalid bundled runtime manifest: {error}"))?;
        let mut devices = Vec::new();
        let mut warnings = Vec::new();
        for backend in [RuntimeBackend::Cuda, RuntimeBackend::Vulkan] {
            let bundled = match backend {
                RuntimeBackend::Cuda => manifest.cuda_sha256.is_some(),
                RuntimeBackend::Vulkan => manifest.vulkan_sha256.is_some(),
                RuntimeBackend::Cpu => false,
            };
            if !bundled {
                continue;
            }
            let executable = match Self::extract_embedded_runtime_for_backend(
                self.storage.data_dir(),
                backend,
            ) {
                Ok(path) => path,
                Err(error) => {
                    warnings.push(format!(
                        "{} runtime is unavailable: {error}",
                        backend.as_str()
                    ));
                    continue;
                }
            };
            match gpu::probe_backend(&executable, backend).await {
                Ok(Some(device)) => devices.push(device),
                Ok(None) => {}
                Err(error) => warnings.push(error),
            }
        }
        *self.compute_devices.write().await = Some(devices.clone());
        *self.compute_device_warnings.write().await = Some(warnings.clone());
        Ok((devices, warnings))
    }

    pub async fn start_huggingface_download(
        self: &Arc<Self>,
        repo_id: String,
        revision: String,
        files: Vec<String>,
    ) -> AppResult<()> {
        let _operation = self.operation_lock.lock().await;
        {
            let mut download_task = self.download_task.lock().await;
            if download_task
                .as_ref()
                .is_some_and(|task| !task.is_finished())
            {
                return Err("A model download is already running".to_string());
            }
            if let Some(task) = download_task.take() {
                task.await
                    .map_err(|error| format!("Previous download task failed: {error}"))?;
            }
        }

        if !matches!(
            *self.runtime_state.read().await,
            RuntimeState::Stopped | RuntimeState::Failed
        ) {
            return Err("Stop the active model before downloading another model".to_string());
        }

        let destination_root = self
            .models_folder
            .read()
            .await
            .clone()
            .filter(|path| path.is_dir())
            .unwrap_or_else(|| self.storage.data_dir().join("models"));
        let (cancel_sender, cancel_receiver) = watch::channel(false);
        *self.download_cancel.lock().await = Some(cancel_sender);
        *self.download_progress.write().await = DownloadProgress {
            state: DownloadState::Preparing,
            repo_id: Some(repo_id.clone()),
            ..DownloadProgress::default()
        };

        let app_state = Arc::clone(self);
        let task = tokio::spawn(async move {
            let download_result = model_hub::download_model(
                &repo_id,
                &revision,
                &files,
                &destination_root,
                Arc::clone(&app_state.download_progress),
                cancel_receiver.clone(),
            )
            .await;

            match download_result {
                Ok(_) => {
                    if let Err(error) = app_state
                        .scan_models(&[destination_root.to_string_lossy().to_string()])
                        .await
                    {
                        app_state
                            .fail_download(format!(
                            "Model was downloaded, but scanning the model folder failed: {error}"
                        ))
                            .await;
                    }
                }
                Err(error) => {
                    let state = if *cancel_receiver.borrow() || error == "Download cancelled" {
                        DownloadState::Cancelled
                    } else {
                        DownloadState::Failed
                    };
                    let mut progress = app_state.download_progress.write().await;
                    progress.state = state;
                    progress.error = (error != "Download cancelled").then_some(error);
                    progress.file_name = None;
                }
            }
            *app_state.download_cancel.lock().await = None;
        });
        *self.download_task.lock().await = Some(task);
        Ok(())
    }

    pub async fn get_huggingface_download_progress(&self) -> DownloadProgress {
        self.download_progress.read().await.clone()
    }

    pub async fn cancel_huggingface_download(&self) -> AppResult<()> {
        let cancel = self.download_cancel.lock().await.clone();
        if let Some(cancel) = cancel {
            cancel
                .send(true)
                .map_err(|error| format!("Could not cancel model download: {error}"))?;
        }
        Ok(())
    }

    async fn fail_download(&self, error: String) {
        let mut progress = self.download_progress.write().await;
        progress.state = DownloadState::Failed;
        progress.error = Some(error);
        progress.file_name = None;
    }

    pub async fn get_capabilities(&self) -> AppResult<serde_json::Value> {
        let capabilities = self.capabilities.read().await;
        serde_json::to_value(capabilities.clone())
            .map_err(|e| format!("Failed to serialize capabilities: {e}"))
    }

    pub fn api_token(&self) -> &str {
        self.token.token()
    }

    pub async fn set_gateway_status(&self, running: bool, error: Option<String>) {
        *self.gateway_running.write().await = running;
        *self.gateway_error.write().await = error;
    }

    pub async fn start_runtime(&self, model_id: &str) -> AppResult<serde_json::Value> {
        let _operation = self.operation_lock.lock().await;
        if self
            .download_task
            .lock()
            .await
            .as_ref()
            .is_some_and(|task| !task.is_finished())
        {
            return Err(
                "Wait for the model download to finish before starting a model".to_string(),
            );
        }
        if runtime_start_is_idempotent(
            *self.runtime_state.read().await,
            self.active_model
                .read()
                .await
                .as_ref()
                .map(|model| model.id.as_str()),
            model_id,
        )? {
            let manifest: RuntimeManifest = serde_json::from_str(RUNTIME_MANIFEST)
                .map_err(|e| format!("Invalid bundled runtime manifest: {e}"))?;
            return Ok(json!({
                "status": "ready",
                "model": model_id,
                "runtime_version": manifest.version,
                "runtime_commit": manifest.source_commit,
                "runtime_backend": self.runtime_backend.read().await.map(|backend| backend.as_str()),
                "already_running": true,
            }));
        }
        let model = self
            .discovered_models
            .read()
            .await
            .iter()
            .find(|model| model.id == model_id)
            .cloned()
            .ok_or_else(|| {
                "Select a discovered GGUF model before starting the runtime".to_string()
            })?;
        let model_path = self.validate_model_path(&model).await?;
        let (devices, hardware_warnings) = self.detect_compute_devices().await?;

        {
            let mut state = self.runtime_state.write().await;
            if !matches!(*state, RuntimeState::Stopped | RuntimeState::Failed) {
                return Err(format!(
                    "Cannot start runtime while it is {}",
                    state.as_str()
                ));
            }
            *state = RuntimeState::Starting;
        }
        *self.runtime_error.write().await = None;
        *self.runtime_warning.write().await = None;
        *self.runtime_backend.write().await = None;
        *self.compute_device.write().await = None;
        *self.active_model.write().await = None;
        let (cancel_sender, cancel_receiver) = watch::channel(false);
        *self.runtime_cancel.lock().await = Some(cancel_sender);

        let manifest: RuntimeManifest = serde_json::from_str(RUNTIME_MANIFEST)
            .map_err(|e| format!("Invalid bundled runtime manifest: {e}"))?;
        let env_allowlist = ["SystemRoot", "PATH", "TEMP", "TMP"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let mut candidate_backends = devices
            .iter()
            .map(|device| device.backend)
            .collect::<Vec<_>>();
        candidate_backends.push(RuntimeBackend::Cpu);
        let mut startup_warnings = hardware_warnings;
        let mut selected_backend = None;
        let mut selected_device = None;
        let mut startup_errors = Vec::new();

        for backend in candidate_backends {
            if *cancel_receiver.borrow() {
                startup_errors.push("Runtime startup was canceled".to_string());
                break;
            }
            let executable = match Self::extract_embedded_runtime_for_backend(
                self.storage.data_dir(),
                backend,
            ) {
                Ok(path) => path,
                Err(error) => {
                    startup_errors.push(format!(
                        "{} runtime is unavailable: {error}",
                        backend.as_str()
                    ));
                    continue;
                }
            };
            let mut args = vec![
                OsString::from("-m"),
                model_path.as_os_str().to_os_string(),
                OsString::from("--alias"),
                OsString::from(&model.id),
                OsString::from("--host"),
                OsString::from("127.0.0.1"),
                OsString::from("--port"),
                OsString::from(self.runtime_port.to_string()),
            ];
            if backend != RuntimeBackend::Cpu {
                args.extend([
                    OsString::from("--gpu-layers"),
                    OsString::from("auto"),
                    OsString::from("--fit"),
                    OsString::from("on"),
                ]);
            }

            let launch_result = async {
                self.supervisor
                    .spawn(
                        RUNTIME_LABEL,
                        executable,
                        args,
                        &env_allowlist,
                        Some(model_path.parent().unwrap_or(Path::new(".")).to_path_buf()),
                    )
                    .await?;
                *self.runtime_state.write().await = RuntimeState::Loading;
                self.wait_for_runtime(cancel_receiver.clone()).await
            }
            .await;

            match launch_result {
                Ok(()) => {
                    selected_backend = Some(backend);
                    selected_device = devices
                        .iter()
                        .find(|device| device.backend == backend)
                        .cloned();
                    break;
                }
                Err(error) => {
                    let diagnostics =
                        match self.supervisor.kill_with_diagnostics(RUNTIME_LABEL).await {
                            Ok(diagnostics) => diagnostics,
                            Err(cleanup_error) => {
                                startup_errors.push(format!(
                                    "{error}; additionally failed to stop runtime: {cleanup_error}"
                                ));
                                break;
                            }
                        };
                    let error = if diagnostics.is_empty() {
                        error
                    } else {
                        format!(
                            "{error}\nRuntime startup diagnostics:\n{}",
                            diagnostics.join("\n")
                        )
                    };
                    if *cancel_receiver.borrow() {
                        startup_errors.push(error);
                        break;
                    }
                    startup_errors.push(format!("{} startup failed: {error}", backend.as_str()));
                }
            }
        }

        *self.runtime_cancel.lock().await = None;
        let Some(backend) = selected_backend else {
            let error = if startup_errors.is_empty() {
                "No compatible runtime backend could be started".to_string()
            } else {
                startup_errors.join("\n\n")
            };
            *self.runtime_state.write().await = RuntimeState::Failed;
            *self.runtime_error.write().await = Some(error.clone());
            return Err(error);
        };

        *self.active_model.write().await = Some(model.clone());
        *self.runtime_backend.write().await = Some(backend);
        *self.compute_device.write().await = selected_device.clone();
        if !startup_errors.is_empty() {
            startup_warnings.extend(startup_errors);
        }
        *self.runtime_warning.write().await =
            (!startup_warnings.is_empty()).then(|| startup_warnings.join("\n"));
        *self.runtime_state.write().await = RuntimeState::Ready;
        Ok(json!({
            "status": "ready",
            "model": model.id,
            "runtime_version": manifest.version,
            "runtime_commit": manifest.source_commit,
            "runtime_backend": backend.as_str(),
            "compute_device": selected_device,
            "warning": self.runtime_warning.read().await.clone(),
        }))
    }

    async fn wait_for_runtime(&self, mut cancel: watch::Receiver<bool>) -> AppResult<()> {
        let health_url = format!("http://127.0.0.1:{}/health", self.runtime_port);
        let deadline = Instant::now() + RUNTIME_READINESS_TIMEOUT;
        let mut last_error = "runtime did not report ready".to_string();
        let mut last_health_error = None;

        loop {
            if !self.supervisor.is_running(RUNTIME_LABEL).await? {
                let detail = last_health_error.as_deref().unwrap_or(last_error.as_str());
                return Err(format!("llama-server exited before readiness: {detail}"));
            }

            let response = tokio::select! {
                _ = cancel.changed() => {
                    if *cancel.borrow() {
                        return Err("Runtime startup was canceled".to_string());
                    }
                    continue;
                }
                response = self.http_client.get(&health_url).send() => response,
            };
            match response {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(response) => {
                    let status = response.status();
                    match response.text().await {
                        Ok(body) if !body.trim().is_empty() => {
                            let detail = body.chars().take(240).collect::<String>();
                            last_error = format!("health endpoint returned {status}: {detail}");
                            last_health_error = Some(last_error.clone());
                        }
                        Ok(_) => {
                            last_error = format!("health endpoint returned {status}");
                            last_health_error = Some(last_error.clone());
                        }
                        Err(error) => {
                            last_error = format!(
                                "health endpoint returned {status}; failed to read response: {error}"
                            );
                            last_health_error = Some(last_error.clone());
                        }
                    }
                }
                Err(error) => {
                    last_error = error.to_string();
                }
            }

            if Instant::now() >= deadline {
                let detail = last_health_error.as_deref().unwrap_or(last_error.as_str());
                return Err(format!(
                    "llama-server did not become ready within {} seconds: {detail}",
                    RUNTIME_READINESS_TIMEOUT.as_secs()
                ));
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    async fn validate_model_path(&self, model: &ModelRecord) -> AppResult<PathBuf> {
        let roots = self.model_roots.read().await.clone();
        let original_metadata = std::fs::symlink_metadata(&model.file_path)
            .map_err(|e| format!("Cannot inspect selected model: {e}"))?;
        if !original_metadata.file_type().is_file() {
            return Err("The selected model must be a regular GGUF file".to_string());
        }
        let path = model
            .file_path
            .canonicalize()
            .map_err(|e| format!("Cannot resolve selected model path: {e}"))?;
        if path.extension().and_then(|ext| ext.to_str()) != Some("gguf") {
            return Err("The selected model must be a regular GGUF file".to_string());
        }
        if is_multimodal_projector(&path) {
            return Err(
                "Это файл визуального проектора mmproj, его нельзя запустить отдельно. \
                 Выберите основной GGUF-файл модели."
                    .to_string(),
            );
        }

        let contained = roots.iter().any(|root| {
            root.canonicalize()
                .map(|canonical_root| path.starts_with(canonical_root))
                .unwrap_or(false)
        });
        if !contained {
            return Err(
                "The selected model is outside the folders scanned by FluxAiBot".to_string(),
            );
        }
        let modified_unix_nanos = modified_unix_nanos(&original_metadata)?;
        if original_metadata.len() != model.size_bytes
            || modified_unix_nanos != model.modified_unix_nanos
        {
            return Err(
                "The model file changed after scanning. Scan the model folder again.".into(),
            );
        }
        Ok(path)
    }

    pub fn extract_embedded_runtime(data_dir: &Path) -> AppResult<PathBuf> {
        Self::extract_embedded_runtime_for_backend(data_dir, RuntimeBackend::Cpu)
    }

    pub fn extract_embedded_runtime_for_backend(
        data_dir: &Path,
        backend: RuntimeBackend,
    ) -> AppResult<PathBuf> {
        let manifest: RuntimeManifest = serde_json::from_str(RUNTIME_MANIFEST)
            .map_err(|e| format!("Invalid bundled runtime manifest: {e}"))?;
        let (binary, expected_sha256) = match backend {
            RuntimeBackend::Cpu => (
                embedded_runtime_resource(EMBEDDED_CPU_RUNTIME_ID)?,
                manifest.sha256.as_str(),
            ),
            RuntimeBackend::Vulkan => (
                embedded_runtime_resource(EMBEDDED_VULKAN_RUNTIME_ID)?,
                manifest
                    .vulkan_sha256
                    .as_deref()
                    .ok_or_else(|| "No Vulkan runtime is bundled".to_string())?,
            ),
            RuntimeBackend::Cuda => {
                let expected_sha256 = manifest
                    .cuda_sha256
                    .as_deref()
                    .ok_or_else(|| "No CUDA runtime is bundled".to_string())?;
                let binary = embedded_runtime_resource(EMBEDDED_CUDA_RUNTIME_ID)?;
                let path = extract_runtime_binary(
                    data_dir,
                    backend,
                    &manifest.version,
                    binary,
                    expected_sha256,
                )?;
                for dependency in &manifest.cuda_dependencies {
                    let binary = embedded_cuda_dependency(&dependency.filename)?;
                    extract_runtime_dependency(
                        data_dir,
                        &dependency.filename,
                        binary,
                        &dependency.sha256,
                    )?;
                }
                return Ok(path);
            }
        };
        extract_runtime_binary(
            data_dir,
            backend,
            &manifest.version,
            binary,
            expected_sha256,
        )
    }

    pub async fn stop_runtime(&self) -> AppResult<serde_json::Value> {
        if let Some(cancel) = self.runtime_cancel.lock().await.as_ref() {
            cancel.send_replace(true);
        }
        let _operation = self.operation_lock.lock().await;
        let mut state = self.runtime_state.write().await;
        if *state == RuntimeState::Stopped {
            return Ok(json!({"status": "stopped"}));
        }
        *state = RuntimeState::Stopping;
        drop(state);

        if let Err(error) = self.supervisor.kill(RUNTIME_LABEL).await {
            *self.runtime_state.write().await = RuntimeState::Failed;
            *self.runtime_error.write().await = Some(error.clone());
            return Err(error);
        }

        *self.active_model.write().await = None;
        *self.runtime_error.write().await = None;
        *self.runtime_warning.write().await = None;
        *self.runtime_backend.write().await = None;
        *self.compute_device.write().await = None;
        *self.runtime_state.write().await = RuntimeState::Stopped;
        Ok(json!({"status": "stopped"}))
    }

    pub async fn scan_models(&self, folders: &[String]) -> AppResult<serde_json::Value> {
        if folders.is_empty() {
            return Err("Select at least one folder to scan for GGUF models".to_string());
        }

        let folders = folders.to_vec();
        let (models, roots) = tokio::task::spawn_blocking(move || scan_model_folders(&folders))
            .await
            .map_err(|e| format!("Model scan task failed: {e}"))??;
        let first_folder = roots.first().cloned();
        if let Some(folder) = &first_folder {
            self.storage
                .set_setting("models_folder", &folder.to_string_lossy())
                .await
                .map_err(|e| format!("Failed to remember the models folder: {e}"))?;
        }
        *self.models_folder.write().await = first_folder;
        *self.model_roots.write().await = roots;
        *self.discovered_models.write().await = models.clone();

        let summaries = models
            .iter()
            .map(|model| {
                json!({
                    "id": model.id,
                    "display_name": model.display_name,
                    "path": model.file_path.to_string_lossy(),
                    "size": model.size_bytes,
                    "role": if is_multimodal_projector(&model.file_path) {
                        "projector"
                    } else {
                        "model"
                    },
                })
            })
            .collect::<Vec<_>>();
        Ok(json!({"models": summaries}))
    }

    pub async fn shutdown(&self) {
        let _ = self.cancel_huggingface_download().await;
        if let Some(task) = self.download_task.lock().await.take() {
            if let Err(error) = task.await {
                eprintln!("Failed to reap model download task during shutdown: {error}");
            }
        }
        if let Err(error) = self.stop_runtime().await {
            eprintln!("Failed to stop llama-server during shutdown: {error}");
        }
        self.storage.close().await;
    }

    pub async fn run_supervisor(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            if *self.runtime_state.read().await != RuntimeState::Ready {
                continue;
            }

            match self.supervisor.is_running(RUNTIME_LABEL).await {
                Ok(true) => {}
                Ok(false) => {
                    *self.runtime_state.write().await = RuntimeState::Failed;
                    *self.active_model.write().await = None;
                    *self.runtime_backend.write().await = None;
                    *self.compute_device.write().await = None;
                    *self.runtime_warning.write().await = None;
                    *self.runtime_error.write().await =
                        Some("llama-server exited unexpectedly".to_string());
                }
                Err(error) => {
                    *self.runtime_state.write().await = RuntimeState::Failed;
                    *self.active_model.write().await = None;
                    *self.runtime_backend.write().await = None;
                    *self.compute_device.write().await = None;
                    *self.runtime_warning.write().await = None;
                    *self.runtime_error.write().await = Some(error);
                }
            }
        }
    }
}

fn scan_model_folders(folders: &[String]) -> AppResult<(Vec<ModelRecord>, Vec<PathBuf>)> {
    let mut models = Vec::new();
    let mut roots = Vec::new();

    for folder in folders {
        let root = PathBuf::from(folder)
            .canonicalize()
            .map_err(|e| format!("Cannot open model folder {folder}: {e}"))?;
        if !root.is_dir() {
            return Err(format!("Model folder is not a directory: {folder}"));
        }
        roots.push(root.clone());

        for entry in walkdir::WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                !entry.file_type().is_dir()
                    || !entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".fluxaibot-")
            })
        {
            let entry = entry.map_err(|e| format!("Failed while scanning {folder}: {e}"))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let is_gguf = entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("gguf"));
            if !is_gguf {
                continue;
            }

            let path = entry
                .path()
                .canonicalize()
                .map_err(|e| format!("Cannot resolve model path: {e}"))?;
            if !path.starts_with(&root) {
                return Err("Model path escaped the selected folder".to_string());
            }
            let metadata = entry
                .metadata()
                .map_err(|e| format!("Cannot read model metadata: {e}"))?;
            let modified_unix_nanos = modified_unix_nanos(&metadata)?;
            let display_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| "Model file name is not valid Unicode".to_string())?
                .to_string();
            let path_hash = Sha256::digest(path.to_string_lossy().as_bytes());
            let id = format!("local-{}", &format!("{path_hash:x}")[..16]);
            models.push(ModelRecord {
                id,
                display_name,
                file_path: path,
                size_bytes: metadata.len(),
                modified_unix_nanos,
                architecture: None,
                quantization: None,
                context_metadata: None,
                supports_chat: false,
                supports_streaming: false,
                supports_tools: false,
                supports_vision: false,
                supports_embeddings: false,
                notes: Some("GGUF metadata and capabilities have not been inspected".to_string()),
                last_verified: None,
            });
        }
    }

    models.sort_by(|left, right| left.display_name.cmp(&right.display_name));
    Ok((models, roots))
}

fn runtime_start_is_idempotent(
    state: RuntimeState,
    active_model_id: Option<&str>,
    requested_model_id: &str,
) -> AppResult<bool> {
    match state {
        RuntimeState::Ready if active_model_id == Some(requested_model_id) => Ok(true),
        RuntimeState::Ready => Err(
            "A different model is already running. Stop it before starting another model."
                .to_string(),
        ),
        RuntimeState::Stopped | RuntimeState::Failed => Ok(false),
        _ => Err(format!(
            "Cannot start runtime while it is {}",
            state.as_str()
        )),
    }
}

fn modified_unix_nanos(metadata: &std::fs::Metadata) -> AppResult<u128> {
    metadata
        .modified()
        .map_err(|e| format!("Cannot read model modification time: {e}"))?
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|e| format!("Model modification time predates the Unix epoch: {e}"))
}

fn is_multimodal_projector(path: &Path) -> bool {
    path.file_name()
        .map(|name| {
            name.to_string_lossy()
                .to_ascii_lowercase()
                .contains("mmproj")
        })
        .unwrap_or(false)
}

fn extract_runtime_binary(
    data_dir: &Path,
    backend: RuntimeBackend,
    version: &str,
    binary: &[u8],
    expected_sha256: &str,
) -> AppResult<PathBuf> {
    let runtime_dir = data_dir.join("runtime");
    let filename = if backend == RuntimeBackend::Cpu {
        format!("llama-server-{version}-{expected_sha256}.exe")
    } else {
        format!(
            "llama-server-{}-{version}-{expected_sha256}.exe",
            backend.as_str()
        )
    };
    let path = runtime_dir.join(filename);
    extract_verified_file(
        &runtime_dir,
        path.file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "Invalid embedded runtime file name".to_string())?,
        binary,
        expected_sha256,
    )?;
    Ok(path)
}

fn extract_runtime_dependency(
    data_dir: &Path,
    filename: &str,
    binary: &[u8],
    expected_sha256: &str,
) -> AppResult<PathBuf> {
    let runtime_dir = data_dir.join("runtime");
    extract_verified_file(&runtime_dir, filename, binary, expected_sha256)?;
    Ok(runtime_dir.join(filename))
}

fn embedded_cuda_dependency(filename: &str) -> AppResult<&'static [u8]> {
    let resource_id = match filename {
        "cublas64_13.dll" => 104,
        "cublasLt64_13.dll" => 105,
        "cudart64_13.dll" => 106,
        "msvcp140.dll" => 107,
        "vcruntime140.dll" => 108,
        "vcruntime140_1.dll" => 109,
        _ => {
            return Err(format!(
                "Unsupported CUDA runtime dependency in manifest: {filename}"
            ))
        }
    };
    embedded_runtime_resource(resource_id)
}

#[cfg(windows)]
fn embedded_runtime_resource(resource_id: u16) -> AppResult<&'static [u8]> {
    let module = unsafe { GetModuleHandleW(std::ptr::null()) };
    if module.is_null() {
        return Err("Cannot locate the FluxAiBot executable module".to_string());
    }

    let name = resource_id as usize as *const u16;
    let resource_type = 10usize as *const u16;
    let resource = unsafe { FindResourceW(module, name, resource_type) };
    if resource.is_null() {
        return Err(format!(
            "Embedded runtime resource {resource_id} was not found"
        ));
    }
    let size = unsafe { SizeofResource(module, resource) };
    if size == 0 {
        return Err(format!("Embedded runtime resource {resource_id} is empty"));
    }
    let loaded = unsafe { LoadResource(module, resource) };
    if loaded.is_null() {
        return Err(format!(
            "Cannot load embedded runtime resource {resource_id}"
        ));
    }
    let data = unsafe { LockResource(loaded) };
    if data.is_null() {
        return Err(format!(
            "Cannot access embedded runtime resource {resource_id}"
        ));
    }
    Ok(unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size as usize) })
}

#[cfg(not(windows))]
fn embedded_runtime_resource(_resource_id: u16) -> AppResult<&'static [u8]> {
    Err("Bundled runtime resources are available only on Windows".to_string())
}

fn extract_verified_file(
    directory: &Path,
    filename: &str,
    binary: &[u8],
    expected_sha256: &str,
) -> AppResult<()> {
    let actual_sha256 = format!("{:x}", Sha256::digest(binary));
    if !actual_sha256.eq_ignore_ascii_case(expected_sha256) {
        return Err(format!(
            "Embedded {filename} failed its SHA-256 integrity check"
        ));
    }
    std::fs::create_dir_all(directory)
        .map_err(|e| format!("Cannot create runtime directory: {e}"))?;
    let path = directory.join(filename);
    if path.is_file() && hash_file(&path)?.eq_ignore_ascii_case(expected_sha256) {
        return Ok(());
    }
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("Cannot replace invalid embedded {filename}: {e}"))?;
    }

    let temporary_path = directory.join(format!(".runtime-{}.tmp", uuid::Uuid::new_v4()));
    let write_result = (|| -> AppResult<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
            .map_err(|e| format!("Cannot create temporary {filename}: {e}"))?;
        file.write_all(binary)
            .map_err(|e| format!("Cannot write embedded {filename}: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("Cannot flush embedded {filename}: {e}"))?;
        Ok(())
    })();
    if let Err(error) = write_result {
        if temporary_path.exists() {
            std::fs::remove_file(&temporary_path).map_err(|cleanup_error| {
                format!("{error}; failed to remove temporary runtime: {cleanup_error}")
            })?;
        }
        return Err(error);
    }
    std::fs::rename(&temporary_path, &path).map_err(|e| {
        let cleanup = std::fs::remove_file(&temporary_path);
        match cleanup {
            Ok(()) => format!("Cannot install embedded {filename}: {e}"),
            Err(cleanup_error) => {
                format!("Cannot install embedded {filename}: {e}; cleanup failed: {cleanup_error}")
            }
        }
    })?;
    Ok(())
}

fn hash_file(path: &Path) -> AppResult<String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("Cannot read model file: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 1024 * 1024];
    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to hash model file: {e}"))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_multimodal_projector_files_without_hiding_main_models() {
        assert!(is_multimodal_projector(Path::new(
            "mmproj-GLM-4.6V-Flash-Q8_0.gguf"
        )));
        assert!(!is_multimodal_projector(Path::new(
            "GLM-4.6V-Flash-Q4_K_M.gguf"
        )));
    }

    #[test]
    fn extracts_embedded_runtime_dependencies_only_after_hash_validation() {
        let data_dir = tempfile::tempdir().expect("create temporary data directory");
        let bytes = b"runtime dependency";
        let hash = format!("{:x}", Sha256::digest(bytes));

        let path = extract_runtime_dependency(data_dir.path(), "sample.dll", bytes, &hash).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(
            extract_runtime_dependency(data_dir.path(), "sample.dll", bytes, &hash).unwrap(),
            path
        );
        std::fs::write(&path, b"corrupted dependency").unwrap();
        extract_runtime_dependency(data_dir.path(), "sample.dll", bytes, &hash).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(extract_runtime_dependency(data_dir.path(), "bad.dll", bytes, "invalid").is_err());
        assert!(!data_dir.path().join("runtime").join("bad.dll").exists());
    }

    #[test]
    fn bundled_cuda_manifest_matches_every_embedded_dependency() {
        let manifest: RuntimeManifest =
            serde_json::from_str(RUNTIME_MANIFEST).expect("parse bundled runtime manifest");
        assert_eq!(manifest.cuda_dependencies.len(), 6);
        for dependency in &manifest.cuda_dependencies {
            let bytes = embedded_cuda_dependency(&dependency.filename)
                .unwrap_or_else(|error| panic!("{error}"));
            let hash = format!("{:x}", Sha256::digest(bytes));
            assert_eq!(hash, dependency.sha256, "{}", dependency.filename);
        }
    }

    #[test]
    fn model_scanning_only_includes_gguf_files_and_keeps_stable_ids() {
        let directory = tempfile::tempdir().unwrap();
        let model_path = directory.path().join("model.gguf");
        let other_path = directory.path().join("weights.bin");
        std::fs::write(&model_path, b"not a valid model, only a scan fixture").unwrap();
        std::fs::write(&other_path, b"ignored").unwrap();

        let folders = vec![directory.path().to_string_lossy().to_string()];
        let (first, roots) = scan_model_folders(&folders).unwrap();
        let (second, _) = scan_model_folders(&folders).unwrap();

        assert_eq!(roots.len(), 1);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].display_name, "model.gguf");
        assert_eq!(first[0].id, second[0].id);
        assert_eq!(
            first[0].size_bytes,
            std::fs::metadata(&model_path).unwrap().len()
        );
        assert_eq!(first[0].modified_unix_nanos, second[0].modified_unix_nanos);
        assert!(!first[0].supports_chat);
        assert!(!first[0].supports_streaming);
    }

    #[test]
    fn model_scan_does_not_read_large_model_contents() {
        let directory = tempfile::tempdir().unwrap();
        let model_path = directory.path().join("large-model.gguf");
        let file = std::fs::File::create(&model_path).unwrap();
        file.set_len(1024 * 1024 * 1024).unwrap();

        let folders = vec![directory.path().to_string_lossy().to_string()];
        let (models, _) = scan_model_folders(&folders).unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].size_bytes, 1024 * 1024 * 1024);
    }

    #[test]
    fn embedded_runtime_is_extracted_and_reused_after_hash_verification() {
        let directory = tempfile::tempdir().unwrap();

        let first = AppState::extract_embedded_runtime(directory.path()).unwrap();
        let second = AppState::extract_embedded_runtime(directory.path()).unwrap();

        assert_eq!(first, second);
        std::fs::write(&first, b"corrupted runtime").unwrap();
        let repaired = AppState::extract_embedded_runtime(directory.path()).unwrap();
        assert_eq!(repaired, first);
        assert_eq!(
            hash_file(&repaired).unwrap(),
            serde_json::from_str::<RuntimeManifest>(RUNTIME_MANIFEST)
                .unwrap()
                .sha256
        );
    }

    #[test]
    fn embedded_vulkan_runtime_is_extracted_and_verified() {
        let directory = tempfile::tempdir().unwrap();
        let manifest: RuntimeManifest = serde_json::from_str(RUNTIME_MANIFEST).unwrap();
        let path = AppState::extract_embedded_runtime_for_backend(
            directory.path(),
            RuntimeBackend::Vulkan,
        )
        .unwrap();

        assert!(path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("vulkan"));
        assert_eq!(hash_file(&path).unwrap(), manifest.vulkan_sha256.unwrap());
    }

    #[tokio::test]
    #[ignore = "requires FLUXAIBOT_E2E_MODEL_DIR and a working GPU backend"]
    async fn gpu_backend_starts_a_model_and_completes_chat() {
        let model_dir = std::env::var_os("FLUXAIBOT_E2E_MODEL_DIR")
            .expect("Set FLUXAIBOT_E2E_MODEL_DIR to a folder containing a small GGUF model");
        let folder = PathBuf::from(model_dir);
        let folder_arg = folder.to_string_lossy().to_string();
        let (models, roots) = scan_model_folders(&[folder_arg]).unwrap();
        let model = models
            .first()
            .expect("The configured folder must contain a launchable GGUF model")
            .clone();
        let state = AppState::new().await.unwrap();
        let listener = std::net::TcpListener::bind(("127.0.0.1", RUNTIME_PORT))
            .expect("The default model runtime port must be available");
        drop(listener);
        *state.model_roots.write().await = roots;
        *state.discovered_models.write().await = models;

        if let Err(error) = state.start_runtime(&model.id).await {
            let cleanup = state.stop_runtime().await;
            panic!("GPU runtime failed to start: {error}; cleanup: {cleanup:?}");
        }
        let backend = *state.runtime_backend.read().await;
        let device = state.compute_device.read().await.clone();
        let status = state.get_status().await.unwrap();
        let response = async {
            let response = tokio::time::timeout(
                Duration::from_secs(120),
                state
                    .http_client
                    .post(format!(
                        "http://127.0.0.1:{}/v1/chat/completions",
                        state.runtime_port
                    ))
                    .json(&json!({
                        "model": model.id,
                        "messages": [{"role": "user", "content": "Reply exactly GPU_E2E_OK"}],
                        "max_tokens": 12,
                        "temperature": 0.0,
                        "stream": false,
                    }))
                    .send(),
            )
            .await
            .map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())?;
            let status = response.status();
            let body = response.text().await.map_err(|error| error.to_string())?;
            if !status.is_success() {
                return Err(format!("Runtime chat returned {status}: {body}"));
            }
            let payload: serde_json::Value =
                serde_json::from_str(&body).map_err(|error| error.to_string())?;
            payload["choices"][0]["message"]["content"]
                .as_str()
                .filter(|content| !content.trim().is_empty())
                .map(str::to_string)
                .ok_or_else(|| "Runtime returned an empty chat response".to_string())
        }
        .await;
        let stop_result = state.stop_runtime().await;
        assert!(
            stop_result.is_ok(),
            "Failed to stop GPU runtime: {stop_result:?}"
        );
        assert!(
            matches!(backend, Some(RuntimeBackend::Cuda | RuntimeBackend::Vulkan)),
            "Expected a GPU backend, got {backend:?}"
        );
        assert!(device.is_some(), "The selected GPU device was not reported");
        assert_eq!(
            status["runtime_backend"].as_str(),
            backend.map(RuntimeBackend::as_str)
        );
        assert!(response.is_ok(), "GPU chat inference failed: {response:?}");
    }

    #[test]
    fn starting_the_active_model_again_is_idempotent() {
        assert!(
            runtime_start_is_idempotent(RuntimeState::Ready, Some("model-a"), "model-a").unwrap()
        );
    }

    #[test]
    fn starting_another_model_while_runtime_is_ready_returns_actionable_error() {
        let error = runtime_start_is_idempotent(RuntimeState::Ready, Some("model-a"), "model-b")
            .unwrap_err();
        assert!(error.contains("Stop it before starting another model"));
    }

    #[test]
    fn model_scan_returns_errors_for_invalid_roots() {
        let folders = vec![r"Z:\missing-model-folder".to_string()];
        assert!(scan_model_folders(&folders).is_err());
    }
}
