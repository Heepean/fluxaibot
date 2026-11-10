//! Codex integration adapter.

use crate::domain::{CompatibilityTier, FeatureState, IntegrationInfo, IntegrationStatus};
use crate::integrations::IntegrationAdapter;
use std::path::PathBuf;

#[derive(Default)]
pub struct CodexAdapter;

impl CodexAdapter {
    pub fn new() -> Self {
        CodexAdapter
    }

    fn detect_codex(&self) -> Option<PathBuf> {
        crate::platform::Platform::find_executable("codex")
            .or_else(|| crate::platform::Platform::find_executable("codex.exe"))
    }
}

impl IntegrationAdapter for CodexAdapter {
    fn app_name(&self) -> &str {
        "Codex"
    }

    fn detect(&self) -> Result<Option<IntegrationInfo>, String> {
        let exe = self.detect_codex();
        let features = vec![
            FeatureState {
                name: "chat".to_string(),
                supported: false,
                note: Some("FluxAiBot does not implement the Codex Responses API".to_string()),
            },
            FeatureState {
                name: "tools".to_string(),
                supported: false,
                note: Some("Not verified; Responses API is not implemented".to_string()),
            },
            FeatureState {
                name: "streaming".to_string(),
                supported: false,
                note: Some("Not verified; Responses API is not implemented".to_string()),
            },
            FeatureState {
                name: "arbitrary_endpoint".to_string(),
                supported: false,
                note: Some("Only documented LM Studio OSS route is supported".to_string()),
            },
        ];

        if let Some(exe) = &exe {
            Ok(Some(IntegrationInfo {
                app: self.app_name().to_string(),
                detected_version: None,
                executable: Some(exe.clone()),
                status: IntegrationStatus::Detected,
                protocol: None,
                last_verified: None,
                notes: Some("Detection only. The documented LM Studio OSS route is separate; FluxAiBot does not configure Codex.".to_string()),
                tier: CompatibilityTier::D,
                features,
            }))
        } else {
            Ok(Some(IntegrationInfo {
                app: self.app_name().to_string(),
                detected_version: None,
                executable: None,
                status: IntegrationStatus::NotDetected,
                protocol: None,
                last_verified: None,
                notes: Some("Codex not found".to_string()),
                tier: CompatibilityTier::D,
                features: vec![],
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codex_detect() {
        let adapter = CodexAdapter::new();
        let info = adapter.detect().unwrap();
        assert_eq!(info.unwrap().app, "Codex");
    }
}
