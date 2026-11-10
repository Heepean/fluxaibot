//! Integration adapters for external coding tools.

pub mod bionic;
pub mod claude_code;
pub mod codex;
pub mod lm_studio;
pub mod opencode;
pub mod vs_code;

use crate::domain::IntegrationInfo;

pub type IntegrationResult<T> = Result<T, String>;

pub trait IntegrationAdapter: Send + Sync {
    fn app_name(&self) -> &str;
    fn detect(&self) -> IntegrationResult<Option<IntegrationInfo>>;
}

pub enum IntegrationAdapterEnum {
    ClaudeCode(claude_code::ClaudeCodeAdapter),
    Codex(codex::CodexAdapter),
    VsCode(vs_code::VsCodeAdapter),
    OpenCode(opencode::OpenCodeAdapter),
    LmStudio(lm_studio::LmStudioIntegrationAdapter),
    Bionic(bionic::BionicAdapter),
}

impl IntegrationAdapter for IntegrationAdapterEnum {
    fn app_name(&self) -> &str {
        match self {
            IntegrationAdapterEnum::ClaudeCode(a) => a.app_name(),
            IntegrationAdapterEnum::Codex(a) => a.app_name(),
            IntegrationAdapterEnum::VsCode(a) => a.app_name(),
            IntegrationAdapterEnum::OpenCode(a) => a.app_name(),
            IntegrationAdapterEnum::LmStudio(a) => a.app_name(),
            IntegrationAdapterEnum::Bionic(a) => a.app_name(),
        }
    }

    fn detect(&self) -> IntegrationResult<Option<IntegrationInfo>> {
        match self {
            IntegrationAdapterEnum::ClaudeCode(a) => a.detect(),
            IntegrationAdapterEnum::Codex(a) => a.detect(),
            IntegrationAdapterEnum::VsCode(a) => a.detect(),
            IntegrationAdapterEnum::OpenCode(a) => a.detect(),
            IntegrationAdapterEnum::LmStudio(a) => a.detect(),
            IntegrationAdapterEnum::Bionic(a) => a.detect(),
        }
    }
}

pub struct IntegrationRegistry {
    adapters: Vec<IntegrationAdapterEnum>,
}

impl IntegrationRegistry {
    pub fn new() -> Self {
        IntegrationRegistry {
            adapters: vec![
                IntegrationAdapterEnum::ClaudeCode(claude_code::ClaudeCodeAdapter::new()),
                IntegrationAdapterEnum::Codex(codex::CodexAdapter::new()),
                IntegrationAdapterEnum::VsCode(vs_code::VsCodeAdapter::new()),
                IntegrationAdapterEnum::OpenCode(opencode::OpenCodeAdapter::new()),
                IntegrationAdapterEnum::LmStudio(lm_studio::LmStudioIntegrationAdapter::new()),
                IntegrationAdapterEnum::Bionic(bionic::BionicAdapter::new()),
            ],
        }
    }

    pub fn discover_all(&self) -> Vec<IntegrationInfo> {
        let mut results = Vec::new();
        for adapter in &self.adapters {
            match adapter.detect() {
                Ok(Some(info)) => results.push(info),
                Ok(None) => results.push(IntegrationInfo {
                    app: adapter.app_name().to_string(),
                    detected_version: None,
                    executable: None,
                    status: crate::domain::IntegrationStatus::NotDetected,
                    protocol: None,
                    last_verified: None,
                    notes: Some("Not detected".to_string()),
                    tier: crate::domain::CompatibilityTier::D,
                    features: vec![],
                }),
                Err(e) => results.push(IntegrationInfo {
                    app: adapter.app_name().to_string(),
                    detected_version: None,
                    executable: None,
                    status: crate::domain::IntegrationStatus::Failed,
                    protocol: None,
                    last_verified: None,
                    notes: Some(format!("Error: {}", e)),
                    tier: crate::domain::CompatibilityTier::D,
                    features: vec![],
                }),
            }
        }
        results
    }
}

impl Default for IntegrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::IntegrationStatus;

    #[test]
    fn discovery_reports_only_verified_capabilities() {
        let registry = IntegrationRegistry::new();
        let infos = registry.discover_all();
        assert_eq!(infos.len(), 6);
        assert!(infos.iter().all(|info| {
            !matches!(
                info.status,
                IntegrationStatus::Configurable
                    | IntegrationStatus::Applying
                    | IntegrationStatus::Configured
                    | IntegrationStatus::Verified
            ) && info.features.iter().all(|feature| !feature.supported)
        }));
    }
}
