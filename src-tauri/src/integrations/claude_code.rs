//! Claude Code integration adapter.

use crate::domain::{CompatibilityTier, FeatureState, IntegrationInfo, IntegrationStatus};
use crate::integrations::IntegrationAdapter;
use std::path::PathBuf;

#[derive(Default)]
pub struct ClaudeCodeAdapter;

impl ClaudeCodeAdapter {
    pub fn new() -> Self {
        ClaudeCodeAdapter
    }

    fn detect_claude(&self) -> Option<PathBuf> {
        crate::platform::Platform::find_executable("claude")
            .or_else(|| crate::platform::Platform::find_executable("claude.exe"))
    }
}

impl IntegrationAdapter for ClaudeCodeAdapter {
    fn app_name(&self) -> &str {
        "Claude Code"
    }

    fn detect(&self) -> Result<Option<IntegrationInfo>, String> {
        let exe = self.detect_claude();

        let features = vec![
            FeatureState {
                name: "chat".to_string(),
                supported: false,
                note: Some("FluxAiBot does not implement Anthropic Messages yet".to_string()),
            },
            FeatureState {
                name: "tools".to_string(),
                supported: false,
                note: Some("Not verified; Messages API is not implemented".to_string()),
            },
            FeatureState {
                name: "streaming".to_string(),
                supported: false,
                note: Some("Not verified; Messages API is not implemented".to_string()),
            },
            FeatureState {
                name: "inline_completions".to_string(),
                supported: false,
                note: Some("Not supported via endpoint override".to_string()),
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
                notes: Some(
                    "Detection only. Messages API setup and live requests are not implemented."
                        .to_string(),
                ),
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
                notes: Some("Claude Code not found".to_string()),
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
    fn test_claude_code_detect() {
        let adapter = ClaudeCodeAdapter::new();
        let info = adapter.detect().unwrap();
        assert_eq!(info.unwrap().app, "Claude Code");
    }
}
