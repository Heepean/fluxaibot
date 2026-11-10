//! VS Code integration adapter.

use crate::domain::{CompatibilityTier, FeatureState, IntegrationInfo, IntegrationStatus};
use crate::integrations::IntegrationAdapter;
use std::path::PathBuf;

#[derive(Default)]
pub struct VsCodeAdapter;

impl VsCodeAdapter {
    pub fn new() -> Self {
        VsCodeAdapter
    }

    fn detect_vscode(&self) -> Option<(PathBuf, String)> {
        let stable = crate::platform::Platform::find_executable("code")
            .or_else(|| crate::platform::Platform::find_executable("code.exe"));
        let insiders = crate::platform::Platform::find_executable("code-insiders")
            .or_else(|| crate::platform::Platform::find_executable("code-insiders.exe"));
        stable
            .map(|p| (p, "stable".to_string()))
            .or_else(|| insiders.map(|p| (p, "insiders".to_string())))
    }
}

impl IntegrationAdapter for VsCodeAdapter {
    fn app_name(&self) -> &str {
        "VS Code"
    }

    fn detect(&self) -> Result<Option<IntegrationInfo>, String> {
        let detected = self.detect_vscode();
        let features = vec![
            FeatureState {
                name: "chat".to_string(),
                supported: false,
                note: Some(
                    "Custom Endpoint setup and API compatibility are not verified".to_string(),
                ),
            },
            FeatureState {
                name: "agent_tools".to_string(),
                supported: false,
                note: Some("Tool calling has not been tested end to end".to_string()),
            },
            FeatureState {
                name: "inline_completions".to_string(),
                supported: false,
                note: Some("Not supported via BYOK Custom Endpoint".to_string()),
            },
            FeatureState {
                name: "embeddings".to_string(),
                supported: false,
                note: Some("Requires Copilot service".to_string()),
            },
        ];

        if let Some((exe, edition)) = detected {
            Ok(Some(IntegrationInfo {
                app: format!("VS Code ({})", edition),
                detected_version: None,
                executable: Some(exe),
                status: IntegrationStatus::Detected,
                protocol: None,
                last_verified: None,
                notes: Some(
                    "Detection only. Custom Endpoint setup is manual and not verified.".to_string(),
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
                notes: Some("VS Code not found".to_string()),
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
    fn test_vscode_detect() {
        let adapter = VsCodeAdapter::new();
        let info = adapter.detect().unwrap();
        assert!(info.unwrap().app.starts_with("VS Code"));
    }
}
