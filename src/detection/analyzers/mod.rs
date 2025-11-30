//! Project analyzers for different project types

pub mod docker;
pub mod env;
pub mod generic;
pub mod git;
pub mod go;
pub mod node;
pub mod python;
pub mod rust;

use super::{DiscoveredScript, DockerService, EnvVarSpec, ExpectedPort, ProjectKind};

/// Result from a project analyzer
#[derive(Debug, Clone)]
pub struct AnalyzerResult {
    pub project_kind: ProjectKind,
    pub confidence: f32,
    pub scripts: Vec<DiscoveredScript>,
    pub env_vars: Vec<EnvVarSpec>,
    pub expected_ports: Vec<ExpectedPort>,
    pub docker_services: Vec<DockerService>,
}

impl AnalyzerResult {
    pub fn new(kind: ProjectKind, confidence: f32) -> Self {
        Self {
            project_kind: kind,
            confidence,
            scripts: Vec::new(),
            env_vars: Vec::new(),
            expected_ports: Vec::new(),
            docker_services: Vec::new(),
        }
    }
}
