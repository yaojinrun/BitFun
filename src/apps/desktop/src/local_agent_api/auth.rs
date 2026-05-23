use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAgentApiAuthConfig {
    pub token: String,
}

pub fn verify_authorization_header(header: Option<&str>, expected_token: &str) -> bool {
    let Some(header) = header else {
        return false;
    };
    let Some(token) = header.strip_prefix("Bearer ") else {
        return false;
    };
    token == expected_token
}

pub async fn load_or_create_token(config_path: PathBuf) -> Result<String> {
    if let Ok(content) = tokio::fs::read_to_string(&config_path).await {
        let config: LocalAgentApiAuthConfig =
            serde_json::from_str(&content).context("Failed to parse Local Agent API config")?;
        if !config.token.trim().is_empty() {
            return Ok(config.token);
        }
    }

    if let Some(parent) = config_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("Failed to create Local Agent API config directory")?;
    }

    let token = uuid::Uuid::new_v4().to_string().replace('-', "");
    let config = LocalAgentApiAuthConfig {
        token: token.clone(),
    };
    let content = serde_json::to_string_pretty(&config)
        .context("Failed to serialize Local Agent API config")?;
    tokio::fs::write(&config_path, content)
        .await
        .context("Failed to write Local Agent API config")?;
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_bearer_token_accepts_exact_value() {
        assert!(verify_authorization_header(
            Some("Bearer abc123"),
            "abc123"
        ));
    }

    #[test]
    fn verify_bearer_token_rejects_missing_or_wrong_value() {
        assert!(!verify_authorization_header(None, "abc123"));
        assert!(!verify_authorization_header(Some("Bearer wrong"), "abc123"));
        assert!(!verify_authorization_header(Some("Basic abc123"), "abc123"));
    }
}
