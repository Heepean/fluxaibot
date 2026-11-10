//! LM Studio Bionic integration adapter.

use crate::domain::{CompatibilityTier, FeatureState, IntegrationInfo, IntegrationStatus};
use crate::integrations::IntegrationAdapter;
use std::path::PathBuf;

#[derive(Default)]
pub struct BionicAdapter;

impl BionicAdapter {
    pub fn new() -> Self {
        BionicAdapter
    }

    fn detect_bionic(&self) -> Option<PathBuf> {
        crate::platform::Platform::find_executable("bionic")
            .or_else(|| crate::platform::Platform::find_executable("bionic.exe"))
    }
}

impl IntegrationAdapter for BionicAdapter {
    fn app_name(&self) -> &str {
        "Bionic"
    }

    fn detect(&self) -> Result<Option<IntegrationInfo>, String> {
        let exe = self.detect_bionic();
        let features = vec![
            FeatureState {
                name: "local_model_library".to_string(),
                supported: false,
                note: Some("Bionic-managed feature; not controlled by FluxAiBot".to_string()),
            },
            FeatureState {
                name: "cloud_models".to_string(),
                supported: false,
                note: Some("Bionic-managed feature; not controlled by FluxAiBot".to_string()),
            },
            FeatureState {
                name: "arbitrary_endpoint".to_string(),
                supported: false,
                note: Some("No public config API for arbitrary endpoint".to_string()),
            },
        ];

        if let Some(exe) = &exe {
            Ok(Some(IntegrationInfo {
                app: self.app_name().to_string(),
                detected_version: None,
                executable: Some(exe.clone()),
                status: IntegrationStatus::ManualActionRequired,
                protocol: None,
                last_verified: None,
                notes: Some(
                    "Detected only. No public arbitrary endpoint config is available.".to_string(),
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
                notes: Some("Bionic not found".to_string()),
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
    fn test_bionic_detect() {
        let adapter = BionicAdapter::new();
        let info = adapter.detect().unwrap();
        assert_eq!(info.unwrap().app, "Bionic");
    }
}
