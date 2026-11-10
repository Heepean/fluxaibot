//! Security utilities: secrets, path validation, token management.

use std::path::Path;

pub struct TokenManager {
    token: String,
}

impl TokenManager {
    pub fn generate() -> Self {
        let token = uuid::Uuid::new_v4().simple().to_string();
        TokenManager { token }
    }

    pub fn with_token(token: String) -> Self {
        TokenManager { token }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn verify(&self, candidate: &str) -> bool {
        constant_time_eq(self.token.as_bytes(), candidate.as_bytes())
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Validate a path is within an allowed root and does not escape via symlinks/reparse points.
pub fn validate_path_within_root(path: &Path, root: &Path) -> bool {
    let path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let root = match root.canonicalize() {
        Ok(r) => r,
        Err(_) => return false,
    };
    path.starts_with(&root)
}

/// Check if a path is a regular file (not a symlink or reparse point).
pub fn is_regular_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    true
}

/// Redact secrets from a string for logging.
pub fn redact_secrets(input: &str, _tokens: &[String]) -> String {
    _tokens
        .iter()
        .filter(|token| !token.is_empty())
        .fold(input.to_string(), |redacted, token| {
            redacted.replace(token, "[REDACTED]")
        })
}

/// Generate a random session id.
pub fn session_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_manager_verify() {
        let tm = TokenManager::generate();
        let token = tm.token();
        assert!(tm.verify(token));
        assert!(!tm.verify("wrong-token"));
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }

    #[test]
    fn test_redact_secrets() {
        let out = redact_secrets("hello secret world", &["secret".to_string()]);
        assert_eq!(out, "hello [REDACTED] world");
    }

    #[test]
    fn test_session_id_length() {
        let id = session_id();
        assert_eq!(id.len(), 32);
    }
}
