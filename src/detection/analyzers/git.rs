//! Git repository analyzer

use anyhow::Result;
use std::path::Path;
use tokio::process::Command;

use crate::detection::GitInfo;

pub async fn analyze(root: &Path) -> Result<Option<GitInfo>> {
    let git_dir = root.join(".git");
    if !git_dir.exists() {
        return Ok(None);
    }

    // Get current branch
    let branch_output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(root)
        .output()
        .await?;

    // Check if git command succeeded
    if !branch_output.status.success() {
        return Ok(None);
    }

    let branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    // Get remote URL
    let remote_output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(root)
        .output()
        .await;

    let remote = remote_output
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    // Get ahead/behind counts
    let status_output = Command::new("git")
        .args(["status", "-sb"])
        .current_dir(root)
        .output()
        .await?;

    let status_line = String::from_utf8_lossy(&status_output.stdout);
    let (ahead, behind) = parse_ahead_behind(&status_line);

    // Check if dirty
    let dirty_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .await?;

    let dirty = !dirty_output.stdout.is_empty();

    Ok(Some(GitInfo {
        branch,
        remote,
        ahead,
        behind,
        dirty,
    }))
}

fn parse_ahead_behind(status: &str) -> (u32, u32) {
    let mut ahead = 0;
    let mut behind = 0;

    if let Some(line) = status.lines().next() {
        // Parse patterns like [ahead 2, behind 1] or [ahead 2] or [behind 1]
        if let Some(bracket_start) = line.find('[') {
            if let Some(bracket_end) = line.find(']') {
                let bracket_content = &line[bracket_start + 1..bracket_end];
                for part in bracket_content.split(", ") {
                    if let Some(num) = part.strip_prefix("ahead ") {
                        ahead = num.parse().unwrap_or(0);
                    } else if let Some(num) = part.strip_prefix("behind ") {
                        behind = num.parse().unwrap_or(0);
                    }
                }
            }
        }
    }

    (ahead, behind)
}
