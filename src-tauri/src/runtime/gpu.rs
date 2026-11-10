use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

const DEVICE_PROBE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_DEVICE_OUTPUT_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeBackend {
    Cuda,
    Vulkan,
    Cpu,
}

impl RuntimeBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cuda => "cuda",
            Self::Vulkan => "vulkan",
            Self::Cpu => "cpu",
        }
    }

    fn device_prefix(self) -> Option<&'static str> {
        match self {
            Self::Cuda => Some("cuda"),
            Self::Vulkan => Some("vulkan"),
            Self::Cpu => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComputeDevice {
    pub backend: RuntimeBackend,
    pub name: String,
    pub total_memory_bytes: Option<u64>,
    pub free_memory_bytes: Option<u64>,
}

pub async fn probe_backend(
    executable: &Path,
    backend: RuntimeBackend,
) -> Result<Option<ComputeDevice>, String> {
    let prefix = backend
        .device_prefix()
        .ok_or_else(|| "CPU runtime does not have a GPU device probe".to_string())?;
    let mut command = Command::new(executable);
    command
        .arg("--list-devices")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .env_clear();
    #[cfg(windows)]
    command.creation_flags(super::process_supervisor::CREATE_NO_WINDOW);
    for key in ["SystemRoot", "PATH", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }

    let output = timeout(DEVICE_PROBE_TIMEOUT, command.output())
        .await
        .map_err(|_| format!("Timed out while checking the {prefix} device"))?
        .map_err(|error| format!("Could not inspect the {prefix} runtime: {error}"))?;
    let output_len = output.stdout.len().saturating_add(output.stderr.len());
    if output_len > MAX_DEVICE_OUTPUT_BYTES {
        return Err(format!(
            "The {prefix} device probe returned too much output ({output_len} bytes)"
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");
    if let Some(device) = parse_device_list(&combined, backend) {
        return Ok(Some(device));
    }
    if output.status.success() {
        return Ok(None);
    }

    let detail = combined
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(3)
        .collect::<Vec<_>>()
        .join("; ");
    Err(if detail.is_empty() {
        format!("The {prefix} runtime device check failed")
    } else {
        format!("The {prefix} runtime device check failed: {detail}")
    })
}

pub fn parse_device_list(output: &str, backend: RuntimeBackend) -> Option<ComputeDevice> {
    let prefix = backend.device_prefix()?;
    for line in output.lines() {
        let Some((device_id, details)) = line.trim().split_once(':') else {
            continue;
        };
        if !device_id.trim().to_ascii_lowercase().starts_with(prefix) {
            continue;
        }
        let details = details.trim();
        if details.is_empty() {
            continue;
        }

        let (name, memory) = match details.split_once('(') {
            Some((name, memory)) => (name.trim(), memory.trim_end_matches(')').trim()),
            None => (details, ""),
        };
        if name.is_empty() {
            continue;
        }

        let mut memory_values = memory.split(',');
        let total_memory_bytes = memory_values.next().and_then(parse_memory_value);
        let free_memory_bytes = memory_values.next().and_then(parse_memory_value);
        return Some(ComputeDevice {
            backend,
            name: name.to_string(),
            total_memory_bytes,
            free_memory_bytes,
        });
    }
    None
}

fn parse_memory_value(value: &str) -> Option<u64> {
    let mut parts = value.split_whitespace();
    let amount = parts.next()?.parse::<u64>().ok()?;
    match parts.next()? {
        "MiB" => Some(amount.saturating_mul(1024 * 1024)),
        "GiB" => Some(amount.saturating_mul(1024 * 1024 * 1024)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vulkan_device_and_free_memory() {
        let device = parse_device_list(
            "Available devices:\n  Vulkan0: AMD Radeon RX 7700 XT (12272 MiB, 11312 MiB free)",
            RuntimeBackend::Vulkan,
        )
        .unwrap();
        assert_eq!(device.name, "AMD Radeon RX 7700 XT");
        assert_eq!(device.total_memory_bytes, Some(12_868_124_672));
        assert_eq!(device.free_memory_bytes, Some(11_861_491_712));
    }

    #[test]
    fn ignores_missing_or_wrong_backend_devices() {
        assert!(
            parse_device_list("Available devices:\n  (none)", RuntimeBackend::Vulkan).is_none()
        );
        assert!(parse_device_list(
            "Available devices:\n  Vulkan0: AMD Radeon RX 7700 XT",
            RuntimeBackend::Cuda
        )
        .is_none());
    }

    #[test]
    fn parses_cuda_device() {
        let device = parse_device_list(
            "CUDA0: NVIDIA GeForce RTX 5090 (32760 MiB, 30000 MiB free)",
            RuntimeBackend::Cuda,
        )
        .unwrap();
        assert_eq!(device.name, "NVIDIA GeForce RTX 5090");
        assert_eq!(device.free_memory_bytes, Some(31_457_280_000));
    }
}
