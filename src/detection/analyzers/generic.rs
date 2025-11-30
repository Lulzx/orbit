//! Generic project analyzer (Makefile, scripts)

use anyhow::Result;
use std::path::Path;

use super::AnalyzerResult;
use crate::detection::{DiscoveredScript, ProjectKind, ScriptCategory, ScriptSource};

pub async fn analyze(root: &Path) -> Result<Option<AnalyzerResult>> {
    let mut scripts = Vec::new();

    // Parse Makefile
    if let Some(makefile_scripts) = parse_makefile(root).await? {
        scripts.extend(makefile_scripts);
    }

    // Check for common script files
    for script in ["run.sh", "start.sh", "build.sh", "deploy.sh", "test.sh"] {
        if root.join(script).exists() {
            let category = if script.contains("test") {
                ScriptCategory::Test
            } else if script.contains("build") {
                ScriptCategory::Build
            } else if script.contains("deploy") {
                ScriptCategory::Deploy
            } else {
                ScriptCategory::Dev
            };

            scripts.push(DiscoveredScript {
                name: script.to_string(),
                command: format!("./{}", script),
                source: ScriptSource::Detected,
                category,
                description: None,
                ports: vec![],
                env_required: vec![],
            });
        }
    }

    // Check scripts directory
    let scripts_dir = root.join("scripts");
    if scripts_dir.is_dir() {
        if let Ok(mut entries) = tokio::fs::read_dir(&scripts_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.ends_with(".sh") {
                        scripts.push(DiscoveredScript {
                            name: name.to_string(),
                            command: format!("./scripts/{}", name),
                            source: ScriptSource::Detected,
                            category: ScriptCategory::Utility,
                            description: None,
                            ports: vec![],
                            env_required: vec![],
                        });
                    }
                }
            }
        }
    }

    if scripts.is_empty() {
        return Ok(None);
    }

    let mut result = AnalyzerResult::new(ProjectKind::Generic, 0.5);
    result.scripts = scripts;

    Ok(Some(result))
}

async fn parse_makefile(root: &Path) -> Result<Option<Vec<DiscoveredScript>>> {
    let makefile_path = if root.join("Makefile").exists() {
        root.join("Makefile")
    } else if root.join("makefile").exists() {
        root.join("makefile")
    } else {
        return Ok(None);
    };

    let content = tokio::fs::read_to_string(&makefile_path).await?;
    let mut scripts = Vec::new();

    for line in content.lines() {
        // Match target definitions like "target:" or "target: deps"
        if let Some(target) = line.strip_suffix(':').or_else(|| {
            line.split(':')
                .next()
                .filter(|s| !s.contains('\t') && !s.starts_with('#') && !s.starts_with('.'))
        }) {
            let target = target.trim();

            // Skip internal targets and empty lines
            if target.is_empty()
                || target.starts_with('.')
                || target.starts_with('#')
                || target.contains(' ')
                || target.contains('$')
            {
                continue;
            }

            let category = categorize_make_target(target);

            scripts.push(DiscoveredScript {
                name: target.to_string(),
                command: format!("make {}", target),
                source: ScriptSource::Makefile,
                category,
                description: None,
                ports: vec![],
                env_required: vec![],
            });
        }
    }

    if scripts.is_empty() {
        return Ok(None);
    }

    Ok(Some(scripts))
}

fn categorize_make_target(target: &str) -> ScriptCategory {
    let target_lower = target.to_lowercase();

    if target_lower.contains("dev")
        || target_lower.contains("run")
        || target_lower.contains("start")
        || target_lower.contains("serve")
    {
        ScriptCategory::Dev
    } else if target_lower.contains("build") || target_lower.contains("compile") {
        ScriptCategory::Build
    } else if target_lower.contains("test") || target_lower.contains("check") {
        ScriptCategory::Test
    } else if target_lower.contains("lint")
        || target_lower.contains("fmt")
        || target_lower.contains("format")
    {
        ScriptCategory::Lint
    } else if target_lower.contains("deploy") || target_lower.contains("release") {
        ScriptCategory::Deploy
    } else if target_lower.contains("db") || target_lower.contains("migrate") {
        ScriptCategory::Database
    } else if target_lower.contains("docker") {
        ScriptCategory::Docker
    } else {
        ScriptCategory::Utility
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn parses_makefile_targets_and_categories() {
        let dir = tempdir().unwrap();
        let makefile = dir.path().join("Makefile");
        std::fs::write(
            &makefile,
            r#"
dev:
	@echo "dev"

build:
	@echo "build"

lint:
	@echo "lint"
        "#,
        )
        .unwrap();

        let result = analyze(dir.path())
            .await
            .expect("analyze should succeed")
            .expect("should detect makefile");

        let mut scripts: Vec<_> = result
            .scripts
            .iter()
            .map(|s| (s.name.clone(), s.category))
            .collect();
        scripts.sort_by(|a, b| a.0.cmp(&b.0));

        assert_eq!(scripts.len(), 3);
        assert!(scripts.contains(&("dev".to_string(), ScriptCategory::Dev)));
        assert!(scripts.contains(&("build".to_string(), ScriptCategory::Build)));
        assert!(scripts.contains(&("lint".to_string(), ScriptCategory::Lint)));
    }
}
