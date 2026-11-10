//! OpenCode integration adapter.

use crate::domain::{CompatibilityTier, FeatureState, IntegrationInfo, IntegrationStatus};
use crate::integrations::IntegrationAdapter;
use std::path::PathBuf;

#[derive(Default)]
pub struct OpenCodeAdapter;

impl OpenCodeAdapter {
    pub fn new() -> Self {
        OpenCodeAdapter
    }

    fn detect_opencode(&self) -> Option<PathBuf> {
        crate::platform::Platform::find_executable("opencode")
            .or_else(|| crate::platform::Platform::find_executable("opencode.exe"))
    }
}

impl IntegrationAdapter for OpenCodeAdapter {
    fn app_name(&self) -> &str {
        "OpenCode"
    }

    fn detect(&self) -> Result<Option<IntegrationInfo>, String> {
        let exe = self.detect_opencode();
        let features = vec![
            FeatureState {
                name: "chat".to_string(),
                supported: false,
                note: Some(
                    "Custom provider configuration is not implemented or verified".to_string(),
                ),
            },
            FeatureState {
                name: "tools".to_string(),
                supported: false,
                note: Some("Tool calling has not been tested end to end".to_string()),
            },
            FeatureState {
                name: "streaming".to_string(),
                supported: false,
                note: Some("API integration has not been tested end to end".to_string()),
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
                    "Detection only. OpenCode provider configuration is not implemented."
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
                notes: Some("OpenCode not found".to_string()),
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
    fn test_opencode_detect() {
        let adapter = OpenCodeAdapter::new();
        let info = adapter.detect().unwrap();
        assert_eq!(info.unwrap().app, "OpenCode");
    }
}
