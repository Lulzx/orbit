//! Environment file analyzer

use anyhow::Result;
use std::path::Path;

/// Result of environment analysis
pub struct EnvAnalysisResult {
    pub dotenv_vars: Vec<String>,
}

pub async fn analyze(root: &Path) -> Result<EnvAnalysisResult> {
    let mut dotenv_vars = Vec::new();

    // Check for various .env files
    let env_files = [
        ".env",
        ".env.local",
        ".env.development",
        ".env.example",
    ];

    for env_file in env_files {
        let path = root.join(env_file);
        if path.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                for line in content.lines() {
                    let line = line.trim();
                    
                    // Skip comments and empty lines
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }

                    // Extract variable name
                    if let Some(eq_pos) = line.find('=') {
                        let var_name = line[..eq_pos].trim().to_string();
                        if !var_name.is_empty() && !dotenv_vars.contains(&var_name) {
                            dotenv_vars.push(var_name);
                        }
                    }
                }
            }
        }
    }

    Ok(EnvAnalysisResult { dotenv_vars })
}
