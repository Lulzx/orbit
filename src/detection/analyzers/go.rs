//! Go project analyzer

use anyhow::Result;
use std::path::Path;

use super::AnalyzerResult;
use crate::detection::{DiscoveredScript, ProjectKind, ScriptCategory, ScriptSource};

pub async fn analyze(root: &Path) -> Result<Option<AnalyzerResult>> {
    let go_mod_path = root.join("go.mod");
    if !go_mod_path.exists() {
        return Ok(None);
    }

    let content = tokio::fs::read_to_string(&go_mod_path).await?;
    
    // Extract module name
    let module_name = content
        .lines()
        .find(|l| l.starts_with("module "))
        .map(|l| l.trim_start_matches("module ").trim().to_string())
        .unwrap_or_else(|| "main".to_string());

    let mut result = AnalyzerResult::new(
        ProjectKind::Go {
            module_name: module_name.clone(),
        },
        0.95,
    );

    // Add standard Go commands
    result.scripts.push(DiscoveredScript {
        name: "run".to_string(),
        command: "go run .".to_string(),
        source: ScriptSource::Detected,
        category: ScriptCategory::Dev,
        description: Some("Run the application".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    result.scripts.push(DiscoveredScript {
        name: "build".to_string(),
        command: "go build -o bin/app .".to_string(),
        source: ScriptSource::Detected,
        category: ScriptCategory::Build,
        description: Some("Build the application".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    result.scripts.push(DiscoveredScript {
        name: "test".to_string(),
        command: "go test ./...".to_string(),
        source: ScriptSource::Detected,
        category: ScriptCategory::Test,
        description: Some("Run all tests".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    result.scripts.push(DiscoveredScript {
        name: "test -v".to_string(),
        command: "go test -v ./...".to_string(),
        source: ScriptSource::Detected,
        category: ScriptCategory::Test,
        description: Some("Run tests with verbose output".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    result.scripts.push(DiscoveredScript {
        name: "fmt".to_string(),
        command: "go fmt ./...".to_string(),
        source: ScriptSource::Detected,
        category: ScriptCategory::Lint,
        description: Some("Format code".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    result.scripts.push(DiscoveredScript {
        name: "vet".to_string(),
        command: "go vet ./...".to_string(),
        source: ScriptSource::Detected,
        category: ScriptCategory::Lint,
        description: Some("Run go vet".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    result.scripts.push(DiscoveredScript {
        name: "mod tidy".to_string(),
        command: "go mod tidy".to_string(),
        source: ScriptSource::Detected,
        category: ScriptCategory::Utility,
        description: Some("Tidy module dependencies".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    result.scripts.push(DiscoveredScript {
        name: "mod download".to_string(),
        command: "go mod download".to_string(),
        source: ScriptSource::Detected,
        category: ScriptCategory::Utility,
        description: Some("Download dependencies".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    // Check for common tools
    if root.join(".golangci.yml").exists() || root.join(".golangci.yaml").exists() {
        result.scripts.push(DiscoveredScript {
            name: "lint".to_string(),
            command: "golangci-lint run".to_string(),
            source: ScriptSource::Detected,
            category: ScriptCategory::Lint,
            description: Some("Run golangci-lint".to_string()),
            ports: vec![],
            env_required: vec![],
        });
    }

    // Check for air (hot reload)
    if root.join(".air.toml").exists() {
        result.scripts.push(DiscoveredScript {
            name: "dev".to_string(),
            command: "air".to_string(),
            source: ScriptSource::Detected,
            category: ScriptCategory::Dev,
            description: Some("Run with hot reload (air)".to_string()),
            ports: vec![],
            env_required: vec![],
        });
    }

    Ok(Some(result))
}
