//! Docker integration - Container management and monitoring

#![allow(dead_code)]

use anyhow::Result;
use bollard::container::{ListContainersOptions, Stats, StatsOptions};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::Docker;
use futures::StreamExt;
use std::path::Path;
use tokio::process::Command;

/// Container information for display
#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: ContainerStatus,
    pub state: String,
    pub ports: Vec<PortMapping>,
    pub stats: Option<ContainerStats>,
    pub created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerStatus {
    Running,
    Paused,
    Exited,
    Created,
    Restarting,
    Dead,
    Unknown,
}

impl std::fmt::Display for ContainerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Exited => write!(f, "exited"),
            Self::Created => write!(f, "created"),
            Self::Restarting => write!(f, "restarting"),
            Self::Dead => write!(f, "dead"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl From<&str> for ContainerStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "running" => Self::Running,
            "paused" => Self::Paused,
            "exited" => Self::Exited,
            "created" => Self::Created,
            "restarting" => Self::Restarting,
            "dead" => Self::Dead,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PortMapping {
    pub private_port: u16,
    pub public_port: Option<u16>,
    pub protocol: String,
}

#[derive(Debug, Clone, Default)]
pub struct ContainerStats {
    pub cpu_percent: f64,
    pub memory_usage_mb: f64,
    pub memory_limit_mb: f64,
    pub memory_percent: f64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

/// Docker client wrapper
pub struct DockerClient {
    docker: Docker,
}

impl DockerClient {
    /// Create a new Docker client
    pub fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Self { docker })
    }

    /// Create with custom socket path
    pub fn with_socket(socket_path: &str) -> Result<Self> {
        let docker = Docker::connect_with_socket(socket_path, 120, bollard::API_DEFAULT_VERSION)?;
        Ok(Self { docker })
    }

    /// Check if Docker is available
    pub async fn is_available(&self) -> bool {
        self.docker.ping().await.is_ok()
    }

    /// List all containers
    pub async fn list_containers(&self, all: bool) -> Result<Vec<ContainerInfo>> {
        let options = ListContainersOptions::<String> {
            all,
            ..Default::default()
        };

        let containers = self.docker.list_containers(Some(options)).await?;
        let mut result = Vec::new();

        for container in containers {
            let id = container.id.unwrap_or_default();
            let name = container
                .names
                .and_then(|n| n.first().cloned())
                .unwrap_or_default()
                .trim_start_matches('/')
                .to_string();

            let image = container.image.unwrap_or_default();
            let state = container.state.unwrap_or_default();
            let status = ContainerStatus::from(state.as_str());

            let ports = container
                .ports
                .unwrap_or_default()
                .into_iter()
                .map(|p| PortMapping {
                    private_port: p.private_port,
                    public_port: p.public_port,
                    protocol: p
                        .typ
                        .map(|t| t.to_string())
                        .unwrap_or_else(|| "tcp".to_string()),
                })
                .collect();

            let created = chrono::DateTime::from_timestamp(container.created.unwrap_or(0), 0)
                .unwrap_or_else(chrono::Utc::now);

            result.push(ContainerInfo {
                id,
                name,
                image,
                status,
                state,
                ports,
                stats: None,
                created,
            });
        }

        Ok(result)
    }

    /// Get stats for a container
    pub async fn get_stats(&self, container_id: &str) -> Result<ContainerStats> {
        let options = StatsOptions {
            stream: false,
            one_shot: true,
        };

        let mut stream = self.docker.stats(container_id, Some(options));

        if let Some(Ok(stats)) = stream.next().await {
            Ok(Self::parse_stats(&stats))
        } else {
            Ok(ContainerStats::default())
        }
    }

    fn parse_stats(stats: &Stats) -> ContainerStats {
        // CPU percentage calculation
        let cpu_delta = stats.cpu_stats.cpu_usage.total_usage as f64
            - stats.precpu_stats.cpu_usage.total_usage as f64;
        let system_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0) as f64
            - stats.precpu_stats.system_cpu_usage.unwrap_or(0) as f64;
        let num_cpus = stats.cpu_stats.online_cpus.unwrap_or(
            stats
                .cpu_stats
                .cpu_usage
                .percpu_usage
                .as_ref()
                .map(|v| v.len())
                .unwrap_or(1) as u64,
        );

        let cpu_percent = if system_delta > 0.0 && cpu_delta > 0.0 {
            let percent = (cpu_delta / system_delta) * num_cpus as f64 * 100.0;
            // Guard against NaN/Inf from edge cases in Docker stats
            if percent.is_nan() || percent.is_infinite() {
                0.0
            } else {
                percent
            }
        } else {
            0.0
        };

        // Memory calculation
        let memory_usage = stats.memory_stats.usage.unwrap_or(0) as f64;
        let memory_limit = stats.memory_stats.limit.unwrap_or(1) as f64;
        let memory_usage_mb = memory_usage / 1024.0 / 1024.0;
        let memory_limit_mb = memory_limit / 1024.0 / 1024.0;
        let memory_percent = (memory_usage / memory_limit) * 100.0;

        // Network stats
        let (network_rx, network_tx) = stats
            .networks
            .as_ref()
            .map(|networks| {
                networks.values().fold((0u64, 0u64), |(rx, tx), net| {
                    (rx + net.rx_bytes, tx + net.tx_bytes)
                })
            })
            .unwrap_or((0, 0));

        ContainerStats {
            cpu_percent,
            memory_usage_mb,
            memory_limit_mb,
            memory_percent,
            network_rx_bytes: network_rx,
            network_tx_bytes: network_tx,
        }
    }

    /// Start a container
    pub async fn start_container(&self, container_id: &str) -> Result<()> {
        self.docker
            .start_container::<String>(container_id, None)
            .await?;
        Ok(())
    }

    /// Stop a container
    pub async fn stop_container(&self, container_id: &str) -> Result<()> {
        self.docker.stop_container(container_id, None).await?;
        Ok(())
    }

    /// Restart a container
    pub async fn restart_container(&self, container_id: &str) -> Result<()> {
        self.docker.restart_container(container_id, None).await?;
        Ok(())
    }

    /// Execute a command in a container
    pub async fn exec_in_container(&self, container_id: &str, cmd: Vec<&str>) -> Result<String> {
        let exec = self
            .docker
            .create_exec(
                container_id,
                CreateExecOptions {
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    cmd: Some(cmd),
                    ..Default::default()
                },
            )
            .await?;

        let mut output = String::new();

        if let StartExecResults::Attached {
            output: mut stream, ..
        } = self.docker.start_exec(&exec.id, None).await?
        {
            while let Some(Ok(msg)) = stream.next().await {
                output.push_str(&msg.to_string());
            }
        }

        Ok(output)
    }

    /// Get container logs
    pub async fn get_logs(&self, container_id: &str, tail: usize) -> Result<Vec<String>> {
        use bollard::container::LogsOptions;

        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            tail: tail.to_string(),
            ..Default::default()
        };

        let mut stream = self.docker.logs(container_id, Some(options));
        let mut logs = Vec::new();

        while let Some(Ok(log)) = stream.next().await {
            logs.push(log.to_string());
        }

        Ok(logs)
    }
}

/// Run docker-compose up
pub async fn compose_up(dir: &Path) -> Result<()> {
    let compose_file = find_compose_file(dir);

    let mut cmd = Command::new("docker");
    cmd.current_dir(dir);
    cmd.arg("compose");

    if let Some(file) = &compose_file {
        cmd.arg("-f").arg(file);
    }

    cmd.arg("up").arg("-d");

    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("docker compose up failed: {}", stderr);
    }

    println!("Docker containers started successfully");
    Ok(())
}

/// Run docker-compose down
pub async fn compose_down(dir: &Path) -> Result<()> {
    let compose_file = find_compose_file(dir);

    let mut cmd = Command::new("docker");
    cmd.current_dir(dir);
    cmd.arg("compose");

    if let Some(file) = &compose_file {
        cmd.arg("-f").arg(file);
    }

    cmd.arg("down");

    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("docker compose down failed: {}", stderr);
    }

    println!("Docker containers stopped successfully");
    Ok(())
}

/// Print Docker status for CLI
pub async fn print_status(_dir: &Path) -> Result<()> {
    let client = DockerClient::new()?;

    if !client.is_available().await {
        println!("Docker is not running or not accessible");
        return Ok(());
    }

    let containers = client.list_containers(true).await?;

    if containers.is_empty() {
        println!("No containers found");
        return Ok(());
    }

    println!("{:<20} {:<25} {:<12} PORTS", "NAME", "IMAGE", "STATUS");
    println!("{}", "-".repeat(80));

    for container in containers {
        let ports: String = container
            .ports
            .iter()
            .filter_map(|p| {
                p.public_port
                    .map(|pub_p| format!("{}:{}", pub_p, p.private_port))
            })
            .collect::<Vec<_>>()
            .join(", ");

        println!(
            "{:<20} {:<25} {:<12} {}",
            truncate(&container.name, 18),
            truncate(&container.image, 23),
            container.status.to_string(),
            ports
        );
    }

    Ok(())
}

fn find_compose_file(dir: &Path) -> Option<String> {
    let candidates = [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ];

    for candidate in candidates {
        if dir.join(candidate).exists() {
            return Some(candidate.to_string());
        }
    }

    None
}

fn truncate(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else if max_len > 3 {
        format!("{}...", s.chars().take(max_len - 3).collect::<String>())
    } else {
        s.chars().take(max_len).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_status_from_str() {
        assert_eq!(ContainerStatus::from("running"), ContainerStatus::Running);
        assert_eq!(ContainerStatus::from("RUNNING"), ContainerStatus::Running);
        assert_eq!(ContainerStatus::from("exited"), ContainerStatus::Exited);
        assert_eq!(
            ContainerStatus::from("unknown_state"),
            ContainerStatus::Unknown
        );
    }
}
