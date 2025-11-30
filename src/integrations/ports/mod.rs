//! Port scanning and management

#![allow(dead_code)]

use anyhow::Result;
use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener};
use tokio::process::Command;

use crate::detection::ProjectDetector;

/// Expected port from project configuration
#[derive(Debug, Clone)]
pub struct ExpectedPort {
    pub port: u16,
    pub source: String,
    pub service_name: String,
}

impl From<crate::detection::ExpectedPort> for ExpectedPort {
    fn from(value: crate::detection::ExpectedPort) -> Self {
        Self {
            port: value.port,
            source: value.source,
            service_name: value.service_name,
        }
    }
}

/// Active port information
#[derive(Debug, Clone)]
pub struct ActivePort {
    pub port: u16,
    pub pid: u32,
    pub process_name: String,
    pub state: PortState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortState {
    Listening,
    Established,
    TimeWait,
    CloseWait,
    Unknown,
}

impl std::fmt::Display for PortState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Listening => write!(f, "LISTEN"),
            Self::Established => write!(f, "ESTABLISHED"),
            Self::TimeWait => write!(f, "TIME_WAIT"),
            Self::CloseWait => write!(f, "CLOSE_WAIT"),
            Self::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Port conflict information
#[derive(Debug, Clone)]
pub struct PortConflict {
    pub port: u16,
    pub expected_service: String,
    pub actual_process: String,
    pub actual_pid: u32,
}

/// Port scan result for events
#[derive(Debug, Clone)]
pub struct PortScanResult {
    pub active_ports: Vec<ActivePort>,
    pub conflicts: Vec<PortConflict>,
}

/// Scan for active ports on the system
pub async fn scan_active_ports() -> Result<Vec<ActivePort>> {
    let output = Command::new("lsof")
        .args(["-iTCP", "-sTCP:LISTEN", "-n", "-P"])
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut ports = Vec::new();
    let mut seen_ports = HashMap::new();

    for line in stdout.lines().skip(1) {
        // Skip header
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 9 {
            continue;
        }

        let process_name = parts[0].to_string();
        let pid: u32 = parts[1].parse().unwrap_or(0);

        // Parse the name field (e.g., "*:3000" or "127.0.0.1:8080")
        let name = parts[8];
        if let Some(port_str) = name.rsplit(':').next() {
            if let Ok(port) = port_str.parse::<u16>() {
                // Deduplicate by port
                if let std::collections::hash_map::Entry::Vacant(e) = seen_ports.entry(port) {
                    e.insert(true);
                    ports.push(ActivePort {
                        port,
                        pid,
                        process_name,
                        state: PortState::Listening,
                    });
                }
            }
        }
    }

    // Sort by port number
    ports.sort_by_key(|p| p.port);

    Ok(ports)
}

/// Scan ports in a specific range
pub async fn scan_port_range(start: u16, end: u16) -> Result<Vec<u16>> {
    let mut open_ports = Vec::new();

    for port in start..=end {
        if is_port_in_use(port) {
            open_ports.push(port);
        }
    }

    Ok(open_ports)
}

/// Check if a specific port is in use
pub fn is_port_in_use(port: u16) -> bool {
    TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).is_err()
}

/// Check if a port is available
pub fn is_port_available(port: u16) -> bool {
    !is_port_in_use(port)
}

/// Find an available port starting from a given port
pub fn find_available_port(start: u16) -> Option<u16> {
    (start..65535).find(|&p| is_port_available(p))
}

/// Kill process on a specific port
pub async fn kill_port(port: u16) -> Result<()> {
    // First, find the PID using lsof
    let output = Command::new("lsof")
        .args(["-ti", &format!(":{}", port)])
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let pids: Vec<&str> = stdout.lines().collect();

    if pids.is_empty() {
        println!("No process found on port {}", port);
        return Ok(());
    }

    for pid_str in pids {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            println!("Killing process {} on port {}", pid, port);

            let kill_output = Command::new("kill")
                .args(["-9", &pid.to_string()])
                .output()
                .await?;

            if !kill_output.status.success() {
                let stderr = String::from_utf8_lossy(&kill_output.stderr);
                eprintln!("Warning: Failed to kill process {}: {}", pid, stderr);
            }
        }
    }

    println!("Port {} freed", port);
    Ok(())
}

/// Get process info for a port
pub async fn get_process_on_port(port: u16) -> Result<Option<(u32, String)>> {
    let output = Command::new("lsof")
        .args(["-i", &format!(":{}", port), "-n", "-P"])
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let process_name = parts[0].to_string();
            let pid: u32 = parts[1].parse().unwrap_or(0);
            return Ok(Some((pid, process_name)));
        }
    }

    Ok(None)
}

/// Detect port conflicts between expected and active ports
pub fn detect_conflicts(expected: &[ExpectedPort], active: &[ActivePort]) -> Vec<PortConflict> {
    let mut conflicts = Vec::new();

    for exp in expected {
        if let Some(active_port) = active.iter().find(|a| a.port == exp.port) {
            // There's a process on this port - check if it's expected
            // For now, we report all occupied expected ports as potential conflicts
            conflicts.push(PortConflict {
                port: exp.port,
                expected_service: exp.service_name.clone(),
                actual_process: active_port.process_name.clone(),
                actual_pid: active_port.pid,
            });
        }
    }

    conflicts
}

/// Print port status for CLI
pub async fn print_port_status(detector: &ProjectDetector) -> Result<()> {
    let context = detector.analyze().await?;
    let active_ports = scan_active_ports().await?;

    println!("Expected Ports:");
    println!("{:<8} {:<20} STATUS", "PORT", "SERVICE");
    println!("{}", "-".repeat(50));

    for expected in &context.ports {
        let status = if let Some(active) = active_ports.iter().find(|a| a.port == expected.port) {
            format!("IN USE by {} (PID: {})", active.process_name, active.pid)
        } else {
            "available".to_string()
        };

        println!(
            "{:<8} {:<20} {}",
            expected.port, expected.service_name, status
        );
    }

    println!("\nActive Ports (listening):");
    println!("{:<8} {:<15} {:<10} STATE", "PORT", "PROCESS", "PID");
    println!("{}", "-".repeat(50));

    for port in &active_ports {
        println!(
            "{:<8} {:<15} {:<10} {}",
            port.port, port.process_name, port.pid, port.state
        );
    }

    Ok(())
}

/// Common development ports to scan
pub const COMMON_DEV_PORTS: &[u16] = &[
    3000, 3001, 3002, 3003, // Common dev servers
    4000, 4200, 4173, // Angular, Vite preview
    5000, 5001, 5173, 5174, // Flask, Vite
    8000, 8080, 8081, 8888, // Common HTTP servers
    9000, 9090,  // Various services
    27017, // MongoDB
    5432,  // PostgreSQL
    3306,  // MySQL
    6379,  // Redis
];

/// Scan common development ports
pub async fn scan_common_ports() -> Result<Vec<ActivePort>> {
    let all_ports = scan_active_ports().await?;
    Ok(all_ports
        .into_iter()
        .filter(|p| COMMON_DEV_PORTS.contains(&p.port))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_availability() {
        // Port 0 should always work (OS assigns one)
        assert!(find_available_port(49152).is_some());
    }

    #[test]
    fn test_detect_conflicts() {
        let expected = vec![ExpectedPort {
            port: 3000,
            source: "package.json".to_string(),
            service_name: "dev-server".to_string(),
        }];

        let active = vec![ActivePort {
            port: 3000,
            pid: 1234,
            process_name: "node".to_string(),
            state: PortState::Listening,
        }];

        let conflicts = detect_conflicts(&expected, &active);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].port, 3000);
    }
}
