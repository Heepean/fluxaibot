//! Public GGUF model discovery and verified Hugging Face downloads.

use crate::runtime::gpu::ComputeDevice;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{Disks, System};
use tokio::io::AsyncWriteExt;
use tokio::sync::{watch, RwLock};
use tokio::time::timeout;
use url::Url;
use uuid::Uuid;

const HF_API: &str = "https://huggingface.co/api";
const DOWNLOAD_READ_TIMEOUT: Duration = Duration::from_secs(60);
const MIN_FREE_SPACE_RESERVE: u64 = 512 * 1024 * 1024;
const MODEL_MEMORY_MULTIPLIER: f64 = 1.5;

pub type HubResult<T> = Result<T, String>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelPurpose {
    Coding,
    Universal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVariant {
    pub label: String,
    pub files: Vec<String>,
    pub size_bytes: u64,
    pub estimated_ram_bytes: u64,
    pub recommended_for_memory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubModel {
    pub repo_id: String,
    pub revision: String,
    pub purpose: ModelPurpose,
    pub downloads: u64,
    pub likes: u64,
    pub license: Option<String>,
    pub variants: Vec<ModelVariant>,
    pub estimated_ram_bytes: u64,
    pub recommended_for_memory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubSearchResult {
    pub models: Vec<HubModel>,
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub total_vram_bytes: u64,
    pub available_vram_bytes: u64,
    pub compute_devices: Vec<ComputeDevice>,
    pub hardware_warnings: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadState {
    Idle,
    Preparing,
    Downloading,
    Verifying,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub state: DownloadState,
    pub repo_id: Option<String>,
    pub file_name: Option<String>,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub destination: Option<String>,
    pub error: Option<String>,
}

impl Default for DownloadProgress {
    fn default() -> Self {
        Self {
            state: DownloadState::Idle,
            repo_id: None,
            file_name: None,
            downloaded_bytes: 0,
            total_bytes: 0,
            destination: None,
            error: None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct HubModelRecord {
    id: String,
    #[serde(default)]
    private: bool,
    #[serde(default)]
    gated: serde_json::Value,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    likes: u64,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    sha: String,
    #[serde(default, rename = "cardData")]
    card_data: Option<HubCardData>,
    #[serde(default)]
    siblings: Vec<HubSibling>,
}

#[derive(Debug, Deserialize)]
struct HubCardData {
    license: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HubSibling {
    rfilename: String,
    size: Option<u64>,
    lfs: Option<HubLfs>,
}

#[derive(Debug, Deserialize)]
struct HubLfs {
    sha256: Option<String>,
    size: Option<u64>,
}

#[derive(Debug, Clone)]
struct DownloadFile {
    name: String,
    size: u64,
    sha256: Option<String>,
}

pub async fn search_models(
    query: &str,
    max_size_bytes: Option<u64>,
    devices: &[ComputeDevice],
) -> HubResult<HubSearchResult> {
    let query = query.trim();
    if query.len() > 160 {
        return Err("Search text must be 160 characters or fewer".to_string());
    }

    let client = metadata_client().await?;
    let mut request = client.get(format!("{HF_API}/models")).query(&[
        ("filter", "gguf"),
        ("pipeline_tag", "text-generation"),
        ("sort", "downloads"),
        ("direction", "-1"),
        ("limit", "16"),
    ]);
    if !query.is_empty() {
        request = request.query(&[("search", query)]);
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("Could not reach Hugging Face model search: {error}"))?;
    let response = checked_response(response, "model search").await?;
    let records: Vec<HubModelRecord> = response
        .json()
        .await
        .map_err(|error| format!("Hugging Face returned invalid model search data: {error}"))?;

    let mut system = System::new();
    system.refresh_memory();
    let total_memory_bytes = system.total_memory();
    let available_memory_bytes = system.available_memory();
    let (total_vram_bytes, available_vram_bytes) = preferred_device_memory(devices);
    let available_model_memory_bytes = available_memory_bytes.saturating_add(available_vram_bytes);
    let client = Arc::new(client);
    let details = futures_util::stream::iter(records.into_iter().filter(|record| {
        record.tags.iter().any(|tag| tag == "gguf") && !record.private && !is_gated(&record.gated)
    }))
    .map(|record| {
        let client = Arc::clone(&client);
        async move {
            let repo_id = record.id.clone();
            let detail = fetch_model_details(&client, &repo_id, None)
                .await
                .map_err(|error| format!("{repo_id}: {error}"))?;
            model_from_record(record, detail, available_model_memory_bytes, max_size_bytes)
        }
    })
    .buffer_unordered(4)
    .collect::<Vec<_>>()
    .await;

    let mut models = Vec::new();
    let mut warnings = Vec::new();
    for detail in details {
        match detail {
            Ok(model) => models.push(model),
            Err(warning) => warnings.push(warning),
        }
    }
    models.sort_by(|left, right| {
        right
            .recommended_for_memory
            .cmp(&left.recommended_for_memory)
            .then_with(|| {
                left.variants
                    .first()
                    .map(|variant| variant.size_bytes)
                    .cmp(&right.variants.first().map(|variant| variant.size_bytes))
            })
            .then_with(|| right.downloads.cmp(&left.downloads))
    });

    Ok(HubSearchResult {
        models,
        total_memory_bytes,
        available_memory_bytes,
        total_vram_bytes,
        available_vram_bytes,
        compute_devices: devices.to_vec(),
        hardware_warnings: Vec::new(),
        warnings,
    })
}

pub async fn download_model(
    repo_id: &str,
    revision: &str,
    requested_files: &[String],
    destination_root: &Path,
    progress: Arc<RwLock<DownloadProgress>>,
    mut cancel: watch::Receiver<bool>,
) -> HubResult<PathBuf> {
    validate_repo_id(repo_id)?;
    validate_revision(revision)?;
    if requested_files.is_empty() || requested_files.len() > 64 {
        return Err("Select between 1 and 64 model files".to_string());
    }
    if requested_files.iter().collect::<HashSet<_>>().len() != requested_files.len() {
        return Err("The selected model contains duplicate file names".to_string());
    }

    set_progress(&progress, |status| status.state = DownloadState::Preparing).await;
    let client = metadata_client().await?;
    let record = fetch_model_details(&client, repo_id, Some(revision)).await?;
    if record.private || is_gated(&record.gated) {
        return Err(
            "Only public, non-gated Hugging Face models can be downloaded here".to_string(),
        );
    }

    let mut files = Vec::with_capacity(requested_files.len());
    for name in requested_files {
        validate_repo_file(name)?;
        let sibling = record
            .siblings
            .iter()
            .find(|sibling| sibling.rfilename == *name)
            .ok_or_else(|| format!("Hugging Face model file was not found: {name}"))?;
        if !is_downloadable_gguf(&sibling.rfilename) {
            return Err(format!(
                "Only standalone GGUF model files can be downloaded: {name}"
            ));
        }
        let size = sibling
            .size
            .or_else(|| sibling.lfs.as_ref().and_then(|lfs| lfs.size))
            .ok_or_else(|| format!("Hugging Face did not report the file size for {name}"))?;
        let sha256 = sibling
            .lfs
            .as_ref()
            .and_then(|lfs| lfs.sha256.as_deref())
            .filter(|value| is_sha256(value))
            .map(str::to_ascii_lowercase);
        files.push(DownloadFile {
            name: name.clone(),
            size,
            sha256,
        });
    }
    if files.iter().any(|file| is_projector(&file.name)) {
        return Err("A mmproj sidecar is not a standalone language model".to_string());
    }
    let total_bytes = files
        .iter()
        .fold(0_u64, |total, file| total.saturating_add(file.size));
    if total_bytes > available_disk_space(destination_root)? {
        return Err("Not enough disk space for the selected model files".to_string());
    }

    set_progress(&progress, |status| {
        status.state = DownloadState::Downloading;
        status.repo_id = Some(repo_id.to_string());
        status.file_name = files.first().map(|file| file.name.clone());
        status.downloaded_bytes = 0;
        status.total_bytes = total_bytes;
        status.error = None;
    })
    .await;

    tokio::fs::create_dir_all(destination_root)
        .await
        .map_err(|error| format!("Cannot create model download folder: {error}"))?;
    let staging_directory = destination_root.join(format!(".fluxaibot-{}.part", Uuid::new_v4()));
    tokio::fs::create_dir(&staging_directory)
        .await
        .map_err(|error| format!("Cannot create temporary download folder: {error}"))?;

    let download_result = download_files(
        &client,
        repo_id,
        revision,
        &files,
        &staging_directory,
        &progress,
        &mut cancel,
    )
    .await;

    if let Err(error) = download_result {
        let cleanup_result = tokio::fs::remove_dir_all(&staging_directory).await;
        if let Err(cleanup_error) = cleanup_result {
            return Err(format!(
                "{error}; failed to remove incomplete download: {cleanup_error}"
            ));
        }
        return Err(error);
    }

    let repo_folder = safe_repo_folder(repo_id);
    let revision_prefix = revision.get(..8).unwrap_or(revision);
    let final_directory = destination_root.join(format!("{repo_folder}-{revision_prefix}"));
    if final_directory.exists() {
        tokio::fs::remove_dir_all(&staging_directory)
            .await
            .map_err(|error| format!("Could not remove temporary download: {error}"))?;
        return Err("This model revision is already downloaded".to_string());
    }
    set_progress(&progress, |status| status.state = DownloadState::Verifying).await;
    tokio::fs::rename(&staging_directory, &final_directory)
        .await
        .map_err(|error| format!("Could not finalize the downloaded model: {error}"))?;

    set_progress(&progress, |status| {
        status.state = DownloadState::Completed;
        status.destination = Some(final_directory.to_string_lossy().to_string());
        status.file_name = None;
    })
    .await;
    Ok(final_directory)
}

async fn download_files(
    client: &reqwest::Client,
    repo_id: &str,
    revision: &str,
    files: &[DownloadFile],
    staging_directory: &Path,
    progress: &Arc<RwLock<DownloadProgress>>,
    cancel: &mut watch::Receiver<bool>,
) -> HubResult<()> {
    let mut downloaded_total = 0_u64;
    for file in files {
        if *cancel.borrow() {
            return Err("Download cancelled".to_string());
        }
        set_progress(progress, |status| {
            status.file_name = Some(file.name.clone())
        })
        .await;
        let url = model_file_url(repo_id, revision, &file.name)?;
        let response = timeout(DOWNLOAD_READ_TIMEOUT, client.get(url).send())
            .await
            .map_err(|_| "Timed out while connecting to the Hugging Face file server".to_string())?
            .map_err(|error| format!("Failed to start model download: {error}"))?;
        let mut response = checked_response(response, "model file").await?;

        if let Some(content_length) = response.content_length() {
            if content_length != file.size {
                return Err(format!(
                    "Hugging Face reported an unexpected file size for {}",
                    file.name
                ));
            }
        }

        let relative_path = safe_relative_path(&file.name)?;
        let output_path = staging_directory.join(relative_path);
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| format!("Could not prepare model file folder: {error}"))?;
        }
        let mut output = tokio::fs::File::create(&output_path)
            .await
            .map_err(|error| format!("Could not create temporary model file: {error}"))?;
        let mut hasher = Sha256::new();
        let mut file_bytes = 0_u64;

        loop {
            let next_chunk = tokio::select! {
                changed = cancel.changed() => {
                    if changed.is_ok() && *cancel.borrow() {
                        return Err("Download cancelled".to_string());
                    }
                    continue;
                }
                result = timeout(DOWNLOAD_READ_TIMEOUT, response.chunk()) => result,
            };
            let chunk = next_chunk
                .map_err(|_| format!("Timed out while downloading {}", file.name))?
                .map_err(|error| format!("Failed while downloading {}: {error}", file.name))?;
            let Some(chunk) = chunk else {
                break;
            };

            file_bytes = file_bytes.saturating_add(chunk.len() as u64);
            if file_bytes > file.size {
                return Err(format!(
                    "Downloaded more bytes than expected for {}",
                    file.name
                ));
            }
            hasher.update(&chunk);
            output
                .write_all(&chunk)
                .await
                .map_err(|error| format!("Could not write model file: {error}"))?;
            let current_total = downloaded_total.saturating_add(file_bytes);
            set_progress(progress, |status| status.downloaded_bytes = current_total).await;
        }

        output
            .flush()
            .await
            .map_err(|error| format!("Could not flush downloaded model file: {error}"))?;
        output
            .sync_all()
            .await
            .map_err(|error| format!("Could not sync downloaded model file: {error}"))?;
        if file_bytes != file.size {
            return Err(format!(
                "Incomplete download for {}: received {file_bytes} of {} bytes",
                file.name, file.size
            ));
        }
        let actual_sha256 = format!("{:x}", hasher.finalize());
        if let Some(expected_sha256) = &file.sha256 {
            if !actual_sha256.eq_ignore_ascii_case(expected_sha256) {
                return Err(format!("SHA-256 verification failed for {}", file.name));
            }
        }
        downloaded_total = downloaded_total.saturating_add(file_bytes);
    }
    Ok(())
}

async fn fetch_model_details(
    client: &reqwest::Client,
    repo_id: &str,
    revision: Option<&str>,
) -> HubResult<HubModelRecord> {
    validate_repo_id(repo_id)?;
    let url = format!("{HF_API}/models/{repo_id}");
    let mut request = client.get(url).query(&[("blobs", "true")]);
    if let Some(revision) = revision {
        request = request.query(&[("revision", revision)]);
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("Could not load Hugging Face model details: {error}"))?;
    checked_response(response, "model details")
        .await?
        .json()
        .await
        .map_err(|error| format!("Hugging Face returned invalid model metadata: {error}"))
}

fn model_from_record(
    summary: HubModelRecord,
    details: HubModelRecord,
    available_memory_bytes: u64,
    max_size_bytes: Option<u64>,
) -> HubResult<HubModel> {
    if details.private || is_gated(&details.gated) {
        return Err(format!("Model {} is not public and ungated", summary.id));
    }
    if summary.id != details.id {
        return Err(
            "Hugging Face model metadata did not match the requested repository".to_string(),
        );
    }
    validate_revision(&details.sha)?;
    let variants = model_variants(&details.siblings, available_memory_bytes);
    let variants = variants
        .into_iter()
        .filter(|variant| max_size_bytes.map_or(true, |limit| variant.size_bytes <= limit))
        .collect::<Vec<_>>();
    if variants.is_empty() {
        return Err(format!(
            "No downloadable GGUF files found for {}",
            summary.id
        ));
    }
    let smallest_size = variants[0].size_bytes;
    let estimated_ram_bytes = estimated_ram(smallest_size);
    let recommended_for_memory = variants
        .iter()
        .any(|variant| variant.recommended_for_memory);
    let license = details
        .card_data
        .as_ref()
        .and_then(|data| data.license.clone())
        .or_else(|| {
            details
                .tags
                .iter()
                .find_map(|tag| tag.strip_prefix("license:").map(str::to_string))
        })
        .or_else(|| {
            summary
                .tags
                .iter()
                .find_map(|tag| tag.strip_prefix("license:").map(str::to_string))
        });

    Ok(HubModel {
        purpose: classify_purpose(&summary.id, &details.tags),
        repo_id: summary.id,
        revision: details.sha,
        downloads: summary.downloads,
        likes: summary.likes,
        license,
        variants,
        estimated_ram_bytes,
        recommended_for_memory,
    })
}

fn classify_purpose(repo_id: &str, tags: &[String]) -> ModelPurpose {
    let has_code_tag = tags.iter().any(|tag| {
        let tag = tag.to_ascii_lowercase();
        matches!(
            tag.as_str(),
            "code" | "coding" | "programming" | "text-generation-code"
        )
    });
    let has_code_name = std::iter::once(repo_id)
        .chain(tags.iter().map(String::as_str))
        .flat_map(|value| {
            value
                .to_ascii_lowercase()
                .split(|character: char| !character.is_ascii_alphanumeric())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .any(|token| {
            matches!(
                token.as_str(),
                "code"
                    | "coder"
                    | "coding"
                    | "starcoder"
                    | "codestral"
                    | "codegemma"
                    | "codellama"
                    | "codeqwen"
                    | "deepseekcoder"
                    | "wizardcoder"
                    | "granitecode"
                    | "stablecode"
            )
        });

    if has_code_tag || has_code_name {
        ModelPurpose::Coding
    } else {
        ModelPurpose::Universal
    }
}

fn model_variants(siblings: &[HubSibling], available_memory_bytes: u64) -> Vec<ModelVariant> {
    let mut singles = Vec::new();
    let mut shards: BTreeMap<String, Vec<(u32, u32, &HubSibling)>> = BTreeMap::new();

    for sibling in siblings {
        if !is_downloadable_gguf(&sibling.rfilename) || is_projector(&sibling.rfilename) {
            continue;
        }
        let Some(size) = sibling
            .size
            .or_else(|| sibling.lfs.as_ref().and_then(|lfs| lfs.size))
        else {
            continue;
        };
        if let Some((prefix, index, total)) = split_shard_name(&sibling.rfilename) {
            shards
                .entry(prefix)
                .or_default()
                .push((index, total, sibling));
        } else {
            singles.push(ModelVariant {
                label: sibling.rfilename.clone(),
                files: vec![sibling.rfilename.clone()],
                size_bytes: size,
                estimated_ram_bytes: estimated_ram(size),
                recommended_for_memory: estimated_ram(size) <= available_memory_bytes,
            });
        }
    }

    for (prefix, mut parts) in shards {
        parts.sort_by_key(|(index, _, _)| *index);
        let Some((_, total, _)) = parts.first() else {
            continue;
        };
        if parts.len() != *total as usize
            || parts
                .iter()
                .enumerate()
                .any(|(index, (part, part_total, _))| {
                    *part != index as u32 + 1 || part_total != total
                })
        {
            continue;
        }
        let files = parts
            .iter()
            .map(|(_, _, sibling)| sibling.rfilename.clone())
            .collect::<Vec<_>>();
        let size_bytes = parts
            .iter()
            .filter_map(|(_, _, sibling)| {
                sibling
                    .size
                    .or_else(|| sibling.lfs.as_ref().and_then(|lfs| lfs.size))
            })
            .fold(0_u64, |total, size| total.saturating_add(size));
        singles.push(ModelVariant {
            label: prefix,
            files,
            size_bytes,
            estimated_ram_bytes: estimated_ram(size_bytes),
            recommended_for_memory: estimated_ram(size_bytes) <= available_memory_bytes,
        });
    }

    singles.sort_by_key(|variant| variant.size_bytes);
    singles
}

fn split_shard_name(path: &str) -> Option<(String, u32, u32)> {
    let filename = Path::new(path).file_name()?.to_str()?;
    let suffix = filename.strip_suffix(".gguf")?;
    let total_separator = suffix.rfind("-of-")?;
    let index_separator = suffix[..total_separator].rfind('-')?;
    let prefix = &suffix[..index_separator];
    let index = suffix[index_separator + 1..total_separator].parse().ok()?;
    let total = suffix[total_separator + 4..].parse().ok()?;
    (index > 0 && total > 1 && index <= total).then(|| {
        (
            format!("{}{}", path.strip_suffix(filename).unwrap_or(""), prefix),
            index,
            total,
        )
    })
}

fn is_gated(gated: &serde_json::Value) -> bool {
    !gated.is_null() && gated != &serde_json::Value::Bool(false)
}

fn is_downloadable_gguf(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("gguf"))
}

fn is_projector(path: &str) -> bool {
    Path::new(path).file_name().is_some_and(|name| {
        name.to_string_lossy()
            .to_ascii_lowercase()
            .contains("mmproj")
    })
}

fn estimated_ram(file_size: u64) -> u64 {
    ((file_size as f64 * MODEL_MEMORY_MULTIPLIER) as u64).saturating_add(1024 * 1024 * 1024)
}

fn preferred_device_memory(devices: &[ComputeDevice]) -> (u64, u64) {
    devices
        .first()
        .map(|device| {
            (
                device.total_memory_bytes.unwrap_or_default(),
                device.free_memory_bytes.unwrap_or_default(),
            )
        })
        .unwrap_or_default()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_repo_id(repo_id: &str) -> HubResult<()> {
    let parts = repo_id.split('/').collect::<Vec<_>>();
    if parts.len() != 2
        || parts.iter().any(|part| {
            part.is_empty() || *part == "." || *part == ".." || !part.bytes().all(is_repo_char)
        })
    {
        return Err("Invalid Hugging Face model repository id".to_string());
    }
    Ok(())
}

fn is_repo_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
}

fn validate_revision(revision: &str) -> HubResult<()> {
    if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Invalid Hugging Face model revision".to_string());
    }
    Ok(())
}

fn validate_repo_file(name: &str) -> HubResult<()> {
    safe_relative_path(name).map(|_| ())
}

fn safe_relative_path(name: &str) -> HubResult<PathBuf> {
    if name.is_empty() || name.contains('\\') || name.contains(':') || name.contains('\0') {
        return Err("Invalid model file path".to_string());
    }
    let path = Path::new(name);
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("Invalid model file path".to_string());
    }
    Ok(path.to_path_buf())
}

fn safe_repo_folder(repo_id: &str) -> String {
    repo_id.replace('/', "--")
}

fn model_file_url(repo_id: &str, revision: &str, file_name: &str) -> HubResult<Url> {
    let mut url = Url::parse("https://huggingface.co/")
        .map_err(|error| format!("Invalid Hugging Face URL: {error}"))?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| "Could not build Hugging Face model file URL".to_string())?;
        segments.pop_if_empty();
        for part in repo_id.split('/') {
            segments.push(part);
        }
        segments.push("resolve").push(revision);
        for part in file_name.split('/') {
            segments.push(part);
        }
    }
    Ok(url)
}

fn available_disk_space(path: &Path) -> HubResult<u64> {
    let disks = Disks::new_with_refreshed_list();
    disks
        .iter()
        .filter(|disk| path.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().components().count())
        .map(|disk| disk.available_space())
        .ok_or_else(|| "Could not determine free space on the model download drive".to_string())
        .and_then(|available| {
            if available <= MIN_FREE_SPACE_RESERVE {
                Err("Not enough free disk space for a safe model download".to_string())
            } else {
                Ok(available - MIN_FREE_SPACE_RESERVE)
            }
        })
}

async fn metadata_client() -> HubResult<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("FluxAiBot/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("Could not initialize Hugging Face client: {error}"))
}

async fn checked_response(
    response: reqwest::Response,
    operation: &str,
) -> HubResult<reqwest::Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|error| format!("Could not read error response: {error}"));
    let detail = body.chars().take(240).collect::<String>();
    Err(format!(
        "Hugging Face {operation} failed ({status}): {detail}"
    ))
}

async fn set_progress(
    progress: &Arc<RwLock<DownloadProgress>>,
    update: impl FnOnce(&mut DownloadProgress),
) {
    update(&mut *progress.write().await);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::gpu::RuntimeBackend;

    fn sibling(name: &str, size: u64) -> HubSibling {
        HubSibling {
            rfilename: name.to_string(),
            size: Some(size),
            lfs: None,
        }
    }

    #[test]
    fn includes_only_complete_gguf_models_and_excludes_projector_sidecars() {
        let siblings = vec![
            sibling("Q4_K_M/model.gguf", 100),
            sibling("Q8_0/model.gguf", 200),
            sibling("mmproj-model.gguf", 20),
            sibling("README.md", 10),
        ];
        let variants = model_variants(&siblings, u64::MAX);
        assert_eq!(variants.len(), 2);
        assert!(variants.iter().all(|variant| variant.files.len() == 1));
        assert!(variants.iter().all(|variant| variant.size_bytes >= 100));
    }

    #[test]
    fn groups_all_shards_and_ignores_incomplete_shard_sets() {
        let siblings = vec![
            sibling("Q4/model-00001-of-00002.gguf", 100),
            sibling("Q4/model-00002-of-00002.gguf", 150),
            sibling("Q5/model-00001-of-00002.gguf", 120),
        ];
        let variants = model_variants(&siblings, u64::MAX);
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].files.len(), 2);
        assert_eq!(variants[0].size_bytes, 250);
    }

    #[test]
    fn estimates_memory_for_each_variant() {
        let variants = model_variants(
            &[
                sibling("small.gguf", 512 * 1024 * 1024),
                sibling("large.gguf", 1024 * 1024 * 1024),
            ],
            2 * 1024 * 1024 * 1024,
        );
        assert!(variants[0].recommended_for_memory);
        assert!(!variants[1].recommended_for_memory);
        assert_eq!(variants[1].estimated_ram_bytes, 5 * 1024 * 1024 * 1024 / 2);
    }

    #[test]
    fn memory_recommendations_use_the_preferred_backend_device_memory() {
        let devices = vec![
            ComputeDevice {
                backend: RuntimeBackend::Cuda,
                name: "GPU A".to_string(),
                total_memory_bytes: Some(12 * 1024 * 1024 * 1024),
                free_memory_bytes: Some(5 * 1024 * 1024 * 1024),
            },
            ComputeDevice {
                backend: RuntimeBackend::Vulkan,
                name: "GPU B".to_string(),
                total_memory_bytes: Some(8 * 1024 * 1024 * 1024),
                free_memory_bytes: Some(8 * 1024 * 1024 * 1024),
            },
        ];

        assert_eq!(
            preferred_device_memory(&devices),
            (12 * 1024 * 1024 * 1024, 5 * 1024 * 1024 * 1024)
        );
    }

    #[test]
    fn rejects_gated_models_and_path_traversal() {
        assert!(is_gated(&serde_json::Value::Bool(true)));
        assert!(!is_gated(&serde_json::Value::Bool(false)));
        assert!(validate_repo_id("../model").is_err());
        assert!(safe_relative_path("../escape.gguf").is_err());
        assert!(safe_relative_path("folder\\model.gguf").is_err());
        assert!(is_sha256(&"a".repeat(64)));
        assert!(!is_sha256("not-a-hash"));
    }

    #[test]
    fn builds_encoded_revision_pinned_model_file_urls() {
        let url = model_file_url(
            "owner/model",
            "0123456789abcdef0123456789abcdef01234567",
            "folder/model file.gguf",
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://huggingface.co/owner/model/resolve/0123456789abcdef0123456789abcdef01234567/folder/model%20file.gguf"
        );
    }

    #[test]
    fn computes_conservative_memory_estimate() {
        assert_eq!(estimated_ram(1024), 1024 * 1024 * 1024 + 1536);
    }

    #[test]
    fn classifies_coding_models_from_names_or_explicit_tags() {
        assert_eq!(
            classify_purpose("Qwen/Qwen2.5-Coder-7B-GGUF", &[]),
            ModelPurpose::Coding
        );
        assert_eq!(
            classify_purpose("author/model-GGUF", &["code".to_string()]),
            ModelPurpose::Coding
        );
        assert_eq!(
            classify_purpose("author/general-chat-GGUF", &[]),
            ModelPurpose::Universal
        );
    }
}
