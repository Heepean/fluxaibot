//! Platform-specific helpers for Windows.

use std::path::PathBuf;

pub struct Platform;

impl Platform {
    pub fn os_version() -> String {
        let mut ver = String::from("Windows 10/11 x64");
        #[cfg(windows)]
        {
            // Read version from registry to avoid windows crate version conflicts.
            let hk = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = hk.open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion") {
                if let Ok(product_name) = subkey.get_value::<String, _>("ProductName") {
                    if let Ok(build) = subkey.get_value::<u32, _>("CurrentBuildNumber") {
                        ver = format!("{} (build {})", product_name, build);
                    }
                }
            }
        }
        ver
    }

    pub fn app_data_dir() -> Result<PathBuf, String> {
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| "Windows LOCALAPPDATA is not available".to_string())?;
        Ok(PathBuf::from(local_app_data).join("FluxAiBot"))
    }

    pub fn find_executable(name: &str) -> Option<PathBuf> {
        which::which(name).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_version_contains_windows() {
        assert!(Platform::os_version().to_lowercase().contains("windows"));
    }

    #[test]
    fn test_app_data_dir() {
        let dir = Platform::app_data_dir();
        assert_eq!(
            dir.unwrap().file_name().and_then(|name| name.to_str()),
            Some("FluxAiBot")
        );
    }
}
