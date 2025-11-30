//! Rust project analyzer

use anyhow::Result;
use std::path::Path;

use super::AnalyzerResult;
use crate::detection::{DiscoveredScript, ProjectKind, ScriptCategory, ScriptSource};

pub async fn analyze(root: &Path) -> Result<Option<AnalyzerResult>> {
    let cargo_path = root.join("Cargo.toml");
    if !cargo_path.exists() {
        return Ok(None);
    }

    let content = tokio::fs::read_to_string(&cargo_path).await?;
    let toml: toml::Value = content.parse()?;

    let is_workspace = toml.get("workspace").is_some();
    let binary_count = toml
        .get("bin")
        .and_then(|b| b.as_array())
        .map(|a| a.len())
        .unwrap_or(1);

    let mut result = AnalyzerResult::new(
        ProjectKind::Rust {
            workspace: is_workspace,
            binary_count,
        },
        0.95,
    );

    // Add standard Cargo commands
    result.scripts.push(DiscoveredScript {
        name: "build".to_string(),
        command: "cargo build".to_string(),
        source: ScriptSource::CargoToml,
        category: ScriptCategory::Build,
        description: Some("Build the project".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    result.scripts.push(DiscoveredScript {
        name: "build --release".to_string(),
        command: "cargo build --release".to_string(),
        source: ScriptSource::CargoToml,
        category: ScriptCategory::Build,
        description: Some("Build for release".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    result.scripts.push(DiscoveredScript {
        name: "run".to_string(),
        command: "cargo run".to_string(),
        source: ScriptSource::CargoToml,
        category: ScriptCategory::Dev,
        description: Some("Run the project".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    result.scripts.push(DiscoveredScript {
        name: "test".to_string(),
        command: "cargo test".to_string(),
        source: ScriptSource::CargoToml,
        category: ScriptCategory::Test,
        description: Some("Run tests".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    result.scripts.push(DiscoveredScript {
        name: "clippy".to_string(),
        command: "cargo clippy".to_string(),
        source: ScriptSource::CargoToml,
        category: ScriptCategory::Lint,
        description: Some("Run clippy lints".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    result.scripts.push(DiscoveredScript {
        name: "fmt".to_string(),
        command: "cargo fmt".to_string(),
        source: ScriptSource::CargoToml,
        category: ScriptCategory::Lint,
        description: Some("Format code".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    result.scripts.push(DiscoveredScript {
        name: "clean".to_string(),
        command: "cargo clean".to_string(),
        source: ScriptSource::CargoToml,
        category: ScriptCategory::Utility,
        description: Some("Clean build artifacts".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    result.scripts.push(DiscoveredScript {
        name: "doc".to_string(),
        command: "cargo doc --open".to_string(),
        source: ScriptSource::CargoToml,
        category: ScriptCategory::Utility,
        description: Some("Generate and open documentation".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    // Add a test action for debugging output streaming
    result.scripts.push(DiscoveredScript {
        name: "echo-test".to_string(),
        command: "echo 'Line 1'; sleep 0.5; echo 'Line 2'; sleep 0.5; echo 'Line 3'; echo 'Done!'"
            .to_string(),
        source: ScriptSource::Detected,
        category: ScriptCategory::Utility,
        description: Some("Test output streaming".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    // Check for common tools in dev-dependencies
    if content.contains("cargo-watch") {
        result.scripts.push(DiscoveredScript {
            name: "watch".to_string(),
            command: "cargo watch -x run".to_string(),
            source: ScriptSource::Detected,
            category: ScriptCategory::Dev,
            description: Some("Watch and run on changes".to_string()),
            ports: vec![],
            env_required: vec![],
        });
    }

    Ok(Some(result))
}
