//! LM Studio integration adapter.

use crate::domain::{CompatibilityTier, FeatureState, IntegrationInfo, IntegrationStatus};
use crate::integrations::IntegrationAdapter;
use std::path::PathBuf;

#[derive(Default)]
pub struct LmStudioIntegrationAdapter;

impl LmStudioIntegrationAdapter {
    pub fn new() -> Self {
        LmStudioIntegrationAdapter
    }

    fn detect_lm_studio(&self) -> Option<PathBuf> {
        crate::platform::Platform::find_executable("lms")
            .or_else(|| crate::platform::Platform::find_executable("lms.exe"))
    }
}

impl IntegrationAdapter for LmStudioIntegrationAdapter {
    fn app_name(&self) -> &str {
        "LM Studio"
    }

    fn detect(&self) -> Result<Option<IntegrationInfo>, String> {
        let exe = self.detect_lm_studio();
        let features = vec![
            FeatureState {
                name: "chat".to_string(),
                supported: false,
                note: Some(
                    "FluxAiBot only detects LM Studio; it does not manage its server".to_string(),
                ),
            },
            FeatureState {
                name: "tools".to_string(),
                supported: false,
                note: Some("Not tested through the FluxAiBot gateway".to_string()),
            },
            FeatureState {
                name: "streaming".to_string(),
                supported: false,
                note: Some("Not tested through the FluxAiBot gateway".to_string()),
            },
            FeatureState {
                name: "responses_api".to_string(),
                supported: false,
                note: Some("FluxAiBot does not implement the Responses API".to_string()),
            },
            FeatureState {
                name: "messages_api".to_string(),
                supported: false,
                note: Some("FluxAiBot does not implement the Anthropic Messages API".to_string()),
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
                    "Detection only. LM Studio server lifecycle is not managed by FluxAiBot."
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
                notes: Some("LM Studio not found".to_string()),
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
    fn test_lm_studio_detect() {
        let adapter = LmStudioIntegrationAdapter::new();
        let info = adapter.detect().unwrap();
        assert_eq!(info.unwrap().app, "LM Studio");
    }
}
