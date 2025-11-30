//! Node.js project analyzer

use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use std::path::Path;

use super::AnalyzerResult;
use crate::detection::{
    DiscoveredScript, EnvVarSpec, ExpectedPort, NodeFramework, PackageManager, ProjectKind,
    ScriptCategory, ScriptSource,
};

// Compile regex once at startup
static PORT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:--port|PORT=|-p)\s*(\d+)").expect("Invalid port regex")
});

#[derive(Deserialize)]
struct PackageJson {
    name: Option<String>,
    scripts: Option<indexmap::IndexMap<String, String>>,
    dependencies: Option<indexmap::IndexMap<String, String>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<indexmap::IndexMap<String, String>>,
}

pub async fn analyze(root: &Path) -> Result<Option<AnalyzerResult>> {
    let pkg_path = root.join("package.json");
    if !pkg_path.exists() {
        return Ok(None);
    }

    let content = tokio::fs::read_to_string(&pkg_path).await?;
    let pkg: PackageJson = serde_json::from_str(&content)?;

    let package_manager = detect_package_manager(root);
    let framework = detect_framework(&pkg);
    let confidence = 0.95;

    let mut result = AnalyzerResult::new(
        ProjectKind::Node {
            package_manager: package_manager.clone(),
            framework: framework.clone(),
        },
        confidence,
    );

    // Parse scripts
    let run_cmd = match &package_manager {
        PackageManager::Npm => "npm run",
        PackageManager::Yarn => "yarn",
        PackageManager::Pnpm => "pnpm run",
        PackageManager::Bun => "bun run",
    };

    if let Some(scripts) = pkg.scripts {
        for (name, command) in scripts {
            let category = categorize_script(&name, &command);
            let ports = extract_ports_from_command(&command);

            result.scripts.push(DiscoveredScript {
                name: name.clone(),
                command: format!("{} {}", run_cmd, name),
                source: ScriptSource::PackageJson,
                category,
                description: None,
                ports,
                env_required: vec![],
            });
        }
    }

    // Add common env vars
    result.env_vars.push(EnvVarSpec {
        name: "NODE_ENV".to_string(),
        description: Some("Node environment mode".to_string()),
        source: "Node.js".to_string(),
        example_value: Some("development".to_string()),
        is_secret: false,
    });

    // Add framework-specific ports
    let default_port = match &framework {
        Some(NodeFramework::NextJs) => 3000,
        Some(NodeFramework::Vite) => 5173,
        Some(NodeFramework::NestJs) => 3000,
        Some(NodeFramework::Astro) => 4321,
        Some(NodeFramework::SvelteKit) => 5173,
        Some(NodeFramework::Nuxt) => 3000,
        _ => 3000,
    };

    result.expected_ports.push(ExpectedPort {
        port: default_port,
        source: "package.json".to_string(),
        service_name: pkg.name.unwrap_or_else(|| "app".to_string()),
    });

    Ok(Some(result))
}

fn detect_package_manager(root: &Path) -> PackageManager {
    if root.join("bun.lockb").exists() {
        PackageManager::Bun
    } else if root.join("pnpm-lock.yaml").exists() {
        PackageManager::Pnpm
    } else if root.join("yarn.lock").exists() {
        PackageManager::Yarn
    } else {
        PackageManager::Npm
    }
}

fn detect_framework(pkg: &PackageJson) -> Option<NodeFramework> {
    let all_deps: Vec<&str> = pkg
        .dependencies
        .iter()
        .flatten()
        .chain(pkg.dev_dependencies.iter().flatten())
        .map(|(k, _)| k.as_str())
        .collect();

    if all_deps.contains(&"next") {
        return Some(NodeFramework::NextJs);
    }
    if all_deps.contains(&"@remix-run/react") {
        return Some(NodeFramework::Remix);
    }
    if all_deps.contains(&"vite") {
        return Some(NodeFramework::Vite);
    }
    if all_deps.contains(&"@nestjs/core") {
        return Some(NodeFramework::NestJs);
    }
    if all_deps.contains(&"astro") {
        return Some(NodeFramework::Astro);
    }
    if all_deps.contains(&"@sveltejs/kit") {
        return Some(NodeFramework::SvelteKit);
    }
    if all_deps.contains(&"nuxt") {
        return Some(NodeFramework::Nuxt);
    }
    if all_deps.contains(&"express") {
        return Some(NodeFramework::Express);
    }

    None
}

fn categorize_script(name: &str, command: &str) -> ScriptCategory {
    let name_lower = name.to_lowercase();
    let cmd_lower = command.to_lowercase();

    if name_lower.contains("dev") || name_lower.contains("start") || name_lower.contains("serve") {
        ScriptCategory::Dev
    } else if name_lower.contains("build") || cmd_lower.contains("build") {
        ScriptCategory::Build
    } else if name_lower.contains("test") || cmd_lower.contains("jest") || cmd_lower.contains("vitest")
    {
        ScriptCategory::Test
    } else if name_lower.contains("lint") || cmd_lower.contains("eslint") || cmd_lower.contains("prettier") {
        ScriptCategory::Lint
    } else if name_lower.contains("deploy") {
        ScriptCategory::Deploy
    } else if name_lower.contains("db") || name_lower.contains("migrate") || name_lower.contains("seed")
    {
        ScriptCategory::Database
    } else if cmd_lower.contains("docker") {
        ScriptCategory::Docker
    } else {
        ScriptCategory::Utility
    }
}

fn extract_ports_from_command(command: &str) -> Vec<u16> {
    let mut ports = Vec::new();

    // Match patterns like --port 3000, -p 3000, PORT=3000
    for cap in PORT_REGEX.captures_iter(command) {
        if let Ok(port) = cap[1].parse::<u16>() {
            ports.push(port);
        }
    }

    ports
}
