//! Project detection and analysis system

#![allow(dead_code)]

pub mod analyzers;

use anyhow::Result;
use std::path::{Path, PathBuf};

// AnalyzerResult is exported for potential external use
#[allow(unused_imports)]
pub use analyzers::AnalyzerResult;

/// Detected project type with confidence score
#[derive(Debug, Clone)]
pub struct ProjectType {
    pub kind: ProjectKind,
    pub confidence: f32,
    pub primary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProjectKind {
    Node {
        package_manager: PackageManager,
        framework: Option<NodeFramework>,
    },
    Rust {
        workspace: bool,
        binary_count: usize,
    },
    Python {
        env_type: PythonEnvType,
        framework: Option<PythonFramework>,
    },
    Go {
        module_name: String,
    },
    Docker {
        compose: bool,
        services: Vec<String>,
    },
    Git,
    Generic,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
    Bun,
}

impl std::fmt::Display for PackageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Npm => write!(f, "npm"),
            Self::Yarn => write!(f, "yarn"),
            Self::Pnpm => write!(f, "pnpm"),
            Self::Bun => write!(f, "bun"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeFramework {
    NextJs,
    Remix,
    Vite,
    Express,
    NestJs,
    Astro,
    SvelteKit,
    Nuxt,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PythonEnvType {
    Venv,
    Poetry,
    Pipenv,
    Conda,
    Uv,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PythonFramework {
    Django,
    Flask,
    FastAPI,
    Starlette,
}

/// Full project context after analysis
#[derive(Debug, Clone)]
pub struct ProjectContext {
    pub root: PathBuf,
    pub name: String,
    pub types: Vec<ProjectType>,
    pub scripts: Vec<DiscoveredScript>,
    pub env_vars: EnvVarAnalysis,
    pub ports: Vec<ExpectedPort>,
    pub docker_services: Vec<DockerService>,
    pub git_info: Option<GitInfo>,
}

/// Discovered runnable script/command
#[derive(Debug, Clone)]
pub struct DiscoveredScript {
    pub name: String,
    pub command: String,
    pub source: ScriptSource,
    pub category: ScriptCategory,
    pub description: Option<String>,
    pub ports: Vec<u16>,
    pub env_required: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptSource {
    PackageJson,
    Makefile,
    CargoToml,
    PyProjectToml,
    DockerCompose,
    OrbitConfig,
    Detected,
}

impl std::fmt::Display for ScriptSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PackageJson => write!(f, "package.json"),
            Self::Makefile => write!(f, "Makefile"),
            Self::CargoToml => write!(f, "Cargo.toml"),
            Self::PyProjectToml => write!(f, "pyproject.toml"),
            Self::DockerCompose => write!(f, "docker-compose"),
            Self::OrbitConfig => write!(f, ".orbit.toml"),
            Self::Detected => write!(f, "detected"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptCategory {
    Dev,
    Build,
    Test,
    Lint,
    Deploy,
    Database,
    Docker,
    Utility,
}

impl std::fmt::Display for ScriptCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dev => write!(f, "dev"),
            Self::Build => write!(f, "build"),
            Self::Test => write!(f, "test"),
            Self::Lint => write!(f, "lint"),
            Self::Deploy => write!(f, "deploy"),
            Self::Database => write!(f, "database"),
            Self::Docker => write!(f, "docker"),
            Self::Utility => write!(f, "utility"),
        }
    }
}

/// Environment variable analysis
#[derive(Debug, Clone, Default)]
pub struct EnvVarAnalysis {
    pub required: Vec<EnvVarSpec>,
    pub optional: Vec<EnvVarSpec>,
    pub set_in_shell: Vec<String>,
    pub set_in_dotenv: Vec<String>,
    pub missing_required: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EnvVarSpec {
    pub name: String,
    pub description: Option<String>,
    pub source: String,
    pub example_value: Option<String>,
    pub is_secret: bool,
}

/// Expected port from configuration
#[derive(Debug, Clone)]
pub struct ExpectedPort {
    pub port: u16,
    pub source: String,
    pub service_name: String,
}

/// Docker service information
#[derive(Debug, Clone)]
pub struct DockerService {
    pub name: String,
    pub image: Option<String>,
    pub ports: Vec<(u16, u16)>,
    pub depends_on: Vec<String>,
}

/// Git repository information
#[derive(Debug, Clone)]
pub struct GitInfo {
    pub branch: String,
    pub remote: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub dirty: bool,
}

/// Main project detector
pub struct ProjectDetector {
    root: PathBuf,
}

impl ProjectDetector {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub async fn analyze(&self) -> Result<ProjectContext> {
        let mut types = Vec::new();
        let mut scripts = Vec::new();
        let mut env_vars = EnvVarAnalysis::default();
        let mut ports = Vec::new();
        let mut docker_services = Vec::new();

        // Node.js detection
        if let Some(result) = analyzers::node::analyze(&self.root).await? {
            types.push(ProjectType {
                kind: result.project_kind,
                confidence: result.confidence,
                primary: types.is_empty(),
            });
            scripts.extend(result.scripts);
            env_vars.required.extend(result.env_vars);
            ports.extend(result.expected_ports);
        }

        // Rust detection
        if let Some(result) = analyzers::rust::analyze(&self.root).await? {
            types.push(ProjectType {
                kind: result.project_kind,
                confidence: result.confidence,
                primary: types.is_empty(),
            });
            scripts.extend(result.scripts);
        }

        // Python detection
        if let Some(result) = analyzers::python::analyze(&self.root).await? {
            types.push(ProjectType {
                kind: result.project_kind,
                confidence: result.confidence,
                primary: types.is_empty(),
            });
            scripts.extend(result.scripts);
            env_vars.required.extend(result.env_vars);
        }

        // Go detection
        if let Some(result) = analyzers::go::analyze(&self.root).await? {
            types.push(ProjectType {
                kind: result.project_kind,
                confidence: result.confidence,
                primary: types.is_empty(),
            });
            scripts.extend(result.scripts);
        }

        // Docker detection
        if let Some(result) = analyzers::docker::analyze(&self.root).await? {
            types.push(ProjectType {
                kind: result.project_kind.clone(),
                confidence: result.confidence,
                primary: types.is_empty(),
            });
            scripts.extend(result.scripts);
            docker_services = result.docker_services;
            ports.extend(result.expected_ports);
        }

        // Git detection
        let git_info = analyzers::git::analyze(&self.root).await?;

        // Generic (Makefile) detection
        if let Some(result) = analyzers::generic::analyze(&self.root).await? {
            scripts.extend(result.scripts);
        }

        // Environment analysis
        let env_result = analyzers::env::analyze(&self.root).await?;
        env_vars.set_in_dotenv = env_result.dotenv_vars;
        env_vars.set_in_shell = std::env::vars().map(|(k, _)| k).collect();

        // Check for missing required vars
        for spec in &env_vars.required {
            if !env_vars.set_in_shell.contains(&spec.name)
                && !env_vars.set_in_dotenv.contains(&spec.name)
            {
                env_vars.missing_required.push(spec.name.clone());
            }
        }

        // Deduplicate scripts by name
        scripts.sort_by(|a, b| a.name.cmp(&b.name));
        scripts.dedup_by(|a, b| a.name == b.name);

        // Detect project name
        let name = self.detect_project_name(&types).await;

        Ok(ProjectContext {
            root: self.root.clone(),
            name,
            types,
            scripts,
            env_vars,
            ports,
            docker_services,
            git_info,
        })
    }

    async fn detect_project_name(&self, _types: &[ProjectType]) -> String {
        // Try package.json name
        if let Ok(content) = tokio::fs::read_to_string(self.root.join("package.json")).await {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(name) = json.get("name").and_then(|n| n.as_str()) {
                    return name.to_string();
                }
            }
        }

        // Try Cargo.toml name
        if let Ok(content) = tokio::fs::read_to_string(self.root.join("Cargo.toml")).await {
            if let Ok(toml) = content.parse::<toml::Value>() {
                if let Some(name) = toml
                    .get("package")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                {
                    return name.to_string();
                }
            }
        }

        // Fall back to directory name
        self.root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project")
            .to_string()
    }

    /// Get all discovered scripts/actions from the project
    pub async fn get_actions(&self) -> Result<Vec<DiscoveredScript>> {
        let context = self.analyze().await?;
        Ok(context.scripts)
    }
}
