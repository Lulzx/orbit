//! Secrets management - macOS Keychain integration

#![allow(dead_code)]

use anyhow::Result;
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;

use crate::detection::ProjectDetector;

const KEYCHAIN_SERVICE: &str = "orbit";

/// Get the keychain account name for a project
fn keychain_account(project_dir: &Path, key: &str) -> String {
    let project_name = project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    format!("{}:{}", project_name, key)
}

/// Store a secret in the macOS Keychain
pub async fn set_secret(project_dir: &Path, key: &str, value: Option<String>) -> Result<()> {
    let value = match value {
        Some(v) => v,
        None => {
            // Prompt for value
            print!("Enter value for '{}': ", key);
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            input.trim().to_string()
        }
    };

    let account = keychain_account(project_dir, key);

    // Delete existing if present
    let _ = delete_generic_password(KEYCHAIN_SERVICE, &account);

    // Set new value
    set_generic_password(KEYCHAIN_SERVICE, &account, value.as_bytes())?;

    println!("Secret '{}' stored in Keychain", key);
    Ok(())
}

/// Get a secret from the macOS Keychain
pub fn get_secret(project_dir: &Path, key: &str) -> Result<Option<String>> {
    let account = keychain_account(project_dir, key);

    match get_generic_password(KEYCHAIN_SERVICE, &account) {
        Ok(password) => {
            let value = String::from_utf8(password.to_vec())?;
            Ok(Some(value))
        }
        Err(_) => Ok(None),
    }
}

/// Remove a secret from the macOS Keychain
pub async fn remove_secret(project_dir: &Path, key: &str) -> Result<()> {
    let account = keychain_account(project_dir, key);

    match delete_generic_password(KEYCHAIN_SERVICE, &account) {
        Ok(_) => {
            println!("Secret '{}' removed from Keychain", key);
        }
        Err(_) => {
            println!("Secret '{}' not found in Keychain", key);
        }
    }

    Ok(())
}

/// List all secrets for a project
pub async fn list_secrets(project_dir: &Path) -> Result<()> {
    let project_name = project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Note: security-framework doesn't have a direct way to list items
    // We'd need to use the Security framework directly or shell out to `security`
    // For now, we'll use the project config to know what secrets might exist

    println!("Secrets for project '{}':", project_name);
    println!("(Use `orbit secrets set <key>` to add secrets)");
    println!();

    // Try to load project config to see expected secrets
    if let Ok(Some(config)) = crate::config::ProjectConfig::load(project_dir) {
        if !config.secrets.keychain.is_empty() {
            println!("Configured keychain secrets:");
            for key in &config.secrets.keychain {
                let status = if get_secret(project_dir, key)?.is_some() {
                    "stored"
                } else {
                    "missing"
                };
                println!("  {} [{}]", key, status);
            }
        }
    }

    Ok(())
}

/// Inject secrets into shell environment
pub async fn inject_secrets(project_dir: &Path, shell: &str) -> Result<()> {
    let mut secrets = HashMap::new();

    // Load project config to get secret keys
    if let Ok(Some(config)) = crate::config::ProjectConfig::load(project_dir) {
        for key in &config.secrets.keychain {
            if let Some(value) = get_secret(project_dir, key)? {
                secrets.insert(key.clone(), value);
            }
        }
    }

    if secrets.is_empty() {
        eprintln!("No secrets configured for this project");
        return Ok(());
    }

    // Output export statements based on shell
    for (key, value) in &secrets {
        let escaped_value = escape_shell_value(value);
        match shell {
            "fish" => println!("set -x {} {}", key, escaped_value),
            _ => println!("export {}={}", key, escaped_value),
        }
    }

    eprintln!();
    eprintln!("# Run the above commands or use: eval $(orbit secrets inject)");

    Ok(())
}

fn escape_shell_value(value: &str) -> String {
    // Simple escaping - wrap in single quotes, escape single quotes
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Print environment variable status
pub async fn print_env_status(detector: &ProjectDetector, show_values: bool) -> Result<()> {
    let context = detector.analyze().await?;

    println!("Environment Variables Status:");
    println!("{:<25} {:<12} {:<15} VALUE", "VARIABLE", "STATUS", "SOURCE");
    println!("{}", "-".repeat(80));

    // Check required variables
    for spec in &context.env_vars.required {
        let (status, source, value) = if let Ok(val) = std::env::var(&spec.name) {
            (
                "set",
                "shell",
                if show_values {
                    if spec.is_secret {
                        mask_value(&val)
                    } else {
                        val
                    }
                } else {
                    "***".to_string()
                },
            )
        } else if context.env_vars.set_in_dotenv.contains(&spec.name) {
            (
                "set",
                ".env",
                if show_values {
                    "from .env".to_string()
                } else {
                    "***".to_string()
                },
            )
        } else {
            ("MISSING", "-", "-".to_string())
        };

        let status_colored = status.to_string();

        println!(
            "{:<25} {:<12} {:<15} {}",
            spec.name, status_colored, source, value
        );
    }

    // Show missing count
    if !context.env_vars.missing_required.is_empty() {
        println!();
        println!(
            "Missing required variables: {}",
            context.env_vars.missing_required.join(", ")
        );
    }

    Ok(())
}

fn mask_value(value: &str) -> String {
    if value.len() <= 4 {
        "*".repeat(value.len())
    } else {
        format!("{}...{}", &value[..2], &value[value.len() - 2..])
    }
}

/// Environment variable source
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvSource {
    Shell,
    DotEnv,
    Keychain,
    Missing,
}

/// Get all environment variables for a project
pub async fn get_project_env(project_dir: &Path) -> Result<HashMap<String, (String, EnvSource)>> {
    let mut env = HashMap::new();

    // Load from .env file
    let dotenv_path = project_dir.join(".env");
    if dotenv_path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&dotenv_path).await {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim().to_string();
                    let value = value
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string();
                    env.insert(key, (value, EnvSource::DotEnv));
                }
            }
        }
    }

    // Load from keychain (configured secrets)
    if let Ok(Some(config)) = crate::config::ProjectConfig::load(project_dir) {
        for key in &config.secrets.keychain {
            if let Some(value) = get_secret(project_dir, key)? {
                env.insert(key.clone(), (value, EnvSource::Keychain));
            }
        }
    }

    // Shell environment (takes precedence)
    for (key, value) in std::env::vars() {
        env.entry(key).or_insert((value, EnvSource::Shell));
    }

    Ok(env)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_value() {
        assert_eq!(mask_value("ab"), "**");
        assert_eq!(mask_value("abcd"), "****");
        assert_eq!(mask_value("abcdefgh"), "ab...gh");
    }

    #[test]
    fn test_escape_shell_value() {
        assert_eq!(escape_shell_value("simple"), "'simple'");
        assert_eq!(escape_shell_value("it's"), "'it'\\''s'");
    }
}
