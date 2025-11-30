//! Docker project analyzer

use anyhow::Result;
use std::path::Path;

use super::AnalyzerResult;
use crate::detection::{
    DiscoveredScript, DockerService, ExpectedPort, ProjectKind, ScriptCategory, ScriptSource,
};

pub async fn analyze(root: &Path) -> Result<Option<AnalyzerResult>> {
    let compose_files = [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ];

    let compose_path = compose_files
        .iter()
        .map(|f| root.join(f))
        .find(|p| p.exists());

    let has_dockerfile = root.join("Dockerfile").exists();

    if compose_path.is_none() && !has_dockerfile {
        return Ok(None);
    }

    let mut services = Vec::new();
    let mut expected_ports = Vec::new();
    let service_names: Vec<String>;

    if let Some(ref path) = compose_path {
        let content = tokio::fs::read_to_string(&path).await?;
        let yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;

        service_names = yaml
            .get("services")
            .and_then(|s| s.as_mapping())
            .map(|m| m.keys().filter_map(|k| k.as_str().map(String::from)).collect())
            .unwrap_or_default();

        // Parse services
        if let Some(svc_map) = yaml.get("services").and_then(|s| s.as_mapping()) {
            for (name, config) in svc_map {
                let name = name.as_str().unwrap_or("unknown").to_string();
                let image = config
                    .get("image")
                    .and_then(|i| i.as_str())
                    .map(String::from);

                let ports: Vec<(u16, u16)> = config
                    .get("ports")
                    .and_then(|p| p.as_sequence())
                    .map(|seq| {
                        seq.iter()
                            .filter_map(|p| {
                                let s = p.as_str()?;
                                let parts: Vec<&str> = s.split(':').collect();
                                if parts.len() >= 2 {
                                    let host = parts[0].parse().ok()?;
                                    let container = parts[1].split('/').next()?.parse().ok()?;
                                    Some((host, container))
                                } else {
                                    None
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                // Add to expected ports
                for (host_port, _) in &ports {
                    expected_ports.push(ExpectedPort {
                        port: *host_port,
                        source: "docker-compose".to_string(),
                        service_name: name.clone(),
                    });
                }

                let depends_on: Vec<String> = config
                    .get("depends_on")
                    .and_then(|d| {
                        if let Some(seq) = d.as_sequence() {
                            Some(
                                seq.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect(),
                            )
                        } else {
                            d.as_mapping().map(|map| {
                                map.keys()
                                    .filter_map(|k| k.as_str().map(String::from))
                                    .collect()
                            })
                        }
                    })
                    .unwrap_or_default();

                services.push(DockerService {
                    name,
                    image,
                    ports,
                    depends_on,
                });
            }
        }
    } else {
        service_names = Vec::new();
    }

    let mut result = AnalyzerResult::new(
        ProjectKind::Docker {
            compose: compose_path.is_some(),
            services: service_names.clone(),
        },
        0.9,
    );

    result.docker_services = services;
    result.expected_ports = expected_ports;

    // Add Docker commands
    if compose_path.is_some() {
        result.scripts.push(DiscoveredScript {
            name: "compose up".to_string(),
            command: "docker compose up -d".to_string(),
            source: ScriptSource::DockerCompose,
            category: ScriptCategory::Docker,
            description: Some("Start all services".to_string()),
            ports: vec![],
            env_required: vec![],
        });

        result.scripts.push(DiscoveredScript {
            name: "compose down".to_string(),
            command: "docker compose down".to_string(),
            source: ScriptSource::DockerCompose,
            category: ScriptCategory::Docker,
            description: Some("Stop all services".to_string()),
            ports: vec![],
            env_required: vec![],
        });

        result.scripts.push(DiscoveredScript {
            name: "compose logs".to_string(),
            command: "docker compose logs -f".to_string(),
            source: ScriptSource::DockerCompose,
            category: ScriptCategory::Docker,
            description: Some("Follow logs".to_string()),
            ports: vec![],
            env_required: vec![],
        });

        result.scripts.push(DiscoveredScript {
            name: "compose ps".to_string(),
            command: "docker compose ps".to_string(),
            source: ScriptSource::DockerCompose,
            category: ScriptCategory::Docker,
            description: Some("List containers".to_string()),
            ports: vec![],
            env_required: vec![],
        });

        result.scripts.push(DiscoveredScript {
            name: "compose restart".to_string(),
            command: "docker compose restart".to_string(),
            source: ScriptSource::DockerCompose,
            category: ScriptCategory::Docker,
            description: Some("Restart all services".to_string()),
            ports: vec![],
            env_required: vec![],
        });

        result.scripts.push(DiscoveredScript {
            name: "compose build".to_string(),
            command: "docker compose build".to_string(),
            source: ScriptSource::DockerCompose,
            category: ScriptCategory::Docker,
            description: Some("Build images".to_string()),
            ports: vec![],
            env_required: vec![],
        });
    }

    if has_dockerfile {
        result.scripts.push(DiscoveredScript {
            name: "docker build".to_string(),
            command: "docker build -t app .".to_string(),
            source: ScriptSource::Detected,
            category: ScriptCategory::Docker,
            description: Some("Build Docker image".to_string()),
            ports: vec![],
            env_required: vec![],
        });
    }

    Ok(Some(result))
}
