//! Lightweight system metrics collector used by the dashboard panels.

use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

/// Snapshot of current system metrics.
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub cpu_percent: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub disk_used_percent: f32,
}

/// Maintains a reusable `sysinfo::System` instance to avoid reallocation on every tick.
pub struct SystemMonitor {
    sys: System,
    disks: Disks,
}

impl SystemMonitor {
    pub fn new() -> Self {
        // Preload the pieces we care about and perform an initial refresh so subsequent
        // samples have stable baselines (especially for CPU usage).
        let mut sys = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        sys.refresh_cpu_usage();
        sys.refresh_memory();

        let mut disks = Disks::new_with_refreshed_list();
        disks.refresh();

        Self { sys, disks }
    }

    /// Refresh and return a metrics snapshot.
    pub fn sample(&mut self) -> SystemMetrics {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.disks.refresh();
        if self.disks.list().is_empty() {
            self.disks.refresh_list();
        }

        let cpu_percent = self.sys.global_cpu_usage();
        let memory_total_mb = self.sys.total_memory() / 1024 / 1024;
        let memory_used_mb = self.sys.used_memory() / 1024 / 1024;

        let (used_bytes, total_bytes) =
            self.disks
                .list()
                .iter()
                .fold((0u128, 0u128), |(used, total), disk| {
                    let total_space = disk.total_space() as u128;
                    let available = disk.available_space() as u128;
                    (
                        used + total_space.saturating_sub(available),
                        total + total_space,
                    )
                });

        let disk_used_percent = if total_bytes > 0 {
            ((used_bytes as f64 / total_bytes as f64) * 100.0) as f32
        } else {
            0.0
        };

        SystemMetrics {
            cpu_percent,
            memory_used_mb,
            memory_total_mb,
            disk_used_percent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_metrics_without_nan() {
        let mut monitor = SystemMonitor::new();
        let metrics = monitor.sample();

        assert!(
            metrics.cpu_percent.is_finite(),
            "CPU percent should always be finite"
        );
        assert!(
            metrics.disk_used_percent.is_finite(),
            "Disk percent should always be finite"
        );
    }
}
