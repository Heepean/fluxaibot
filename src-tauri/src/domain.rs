//! Domain types shared across the application.

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecord {
    pub id: String,
    pub display_name: String,
    pub file_path: PathBuf,
    pub size_bytes: u64,
    pub modified_unix_nanos: u128,
    pub architecture: Option<String>,
    pub quantization: Option<String>,
    pub context_metadata: Option<u64>,
    pub supports_chat: bool,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_embeddings: bool,
    pub notes: Option<String>,
    pub last_verified: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeState {
    Stopped,
    Starting,
    Loading,
    Ready,
    Stopping,
    Failed,
}

impl RuntimeState {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeState::Stopped => "stopped",
            RuntimeState::Starting => "starting",
            RuntimeState::Loading => "loading",
            RuntimeState::Ready => "ready",
            RuntimeState::Stopping => "stopping",
            RuntimeState::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrationStatus {
    NotDetected,
    Detected,
    Configurable,
    Applying,
    Configured,
    Verified,
    Failed,
    ManualActionRequired,
    Unsupported,
}

impl IntegrationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            IntegrationStatus::NotDetected => "not_detected",
            IntegrationStatus::Detected => "detected",
            IntegrationStatus::Configurable => "configurable",
            IntegrationStatus::Applying => "applying",
            IntegrationStatus::Configured => "configured",
            IntegrationStatus::Verified => "verified",
            IntegrationStatus::Failed => "failed",
            IntegrationStatus::ManualActionRequired => "manual_action_required",
            IntegrationStatus::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationInfo {
    pub app: String,
    pub detected_version: Option<String>,
    pub executable: Option<PathBuf>,
    pub status: IntegrationStatus,
    pub protocol: Option<String>,
    pub last_verified: Option<String>,
    pub notes: Option<String>,
    pub tier: CompatibilityTier,
    pub features: Vec<FeatureState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompatibilityTier {
    A,
    B,
    C,
    D,
}

impl CompatibilityTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            CompatibilityTier::A => "A",
            CompatibilityTier::B => "B",
            CompatibilityTier::C => "C",
            CompatibilityTier::D => "D",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureState {
    pub name: String,
    pub supported: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityDocument {
    pub protocols: Vec<String>,
    pub endpoints: Vec<String>,
    pub model_capabilities: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DomainError {
    pub code: String,
    pub message: String,
    pub diagnostic: Option<String>,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl Error for DomainError {}

impl DomainError {
    pub fn new(code: &str, message: &str) -> Self {
        DomainError {
            code: code.to_string(),
            message: message.to_string(),
            diagnostic: None,
        }
    }

    pub fn with_diagnostic(mut self, diag: &str) -> Self {
        self.diagnostic = Some(diag.to_string());
        self
    }
}

pub type DomainResult<T> = Result<T, DomainError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_state_as_str() {
        assert_eq!(RuntimeState::Ready.as_str(), "ready");
        assert_eq!(RuntimeState::Stopped.as_str(), "stopped");
    }

    #[test]
    fn test_integration_status_as_str() {
        assert_eq!(IntegrationStatus::Verified.as_str(), "verified");
        assert_eq!(IntegrationStatus::Unsupported.as_str(), "unsupported");
    }

    #[test]
    fn test_compatibility_tier_as_str() {
        assert_eq!(CompatibilityTier::A.as_str(), "A");
        assert_eq!(CompatibilityTier::D.as_str(), "D");
    }

    #[test]
    fn test_domain_error_display() {
        let err = DomainError::new("E001", "test error");
        assert_eq!(format!("{err}"), "[E001] test error");
    }
}
