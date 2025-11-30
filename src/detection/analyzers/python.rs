//! Python project analyzer

use anyhow::Result;
use std::path::Path;

use super::AnalyzerResult;
use crate::detection::{
    DiscoveredScript, EnvVarSpec, ExpectedPort, ProjectKind, PythonEnvType, PythonFramework,
    ScriptCategory, ScriptSource,
};

pub async fn analyze(root: &Path) -> Result<Option<AnalyzerResult>> {
    // Check for Python project indicators
    let has_pyproject = root.join("pyproject.toml").exists();
    let has_requirements = root.join("requirements.txt").exists();
    let has_setup_py = root.join("setup.py").exists();
    let has_pipfile = root.join("Pipfile").exists();

    if !has_pyproject && !has_requirements && !has_setup_py && !has_pipfile {
        return Ok(None);
    }

    let env_type = detect_env_type(root);
    let framework = detect_framework(root).await?;

    let mut result = AnalyzerResult::new(
        ProjectKind::Python {
            env_type: env_type.clone(),
            framework: framework.clone(),
        },
        0.9,
    );

    // Add environment activation
    match &env_type {
        PythonEnvType::Poetry => {
            result.scripts.push(DiscoveredScript {
                name: "shell".to_string(),
                command: "poetry shell".to_string(),
                source: ScriptSource::Detected,
                category: ScriptCategory::Dev,
                description: Some("Activate Poetry environment".to_string()),
                ports: vec![],
                env_required: vec![],
            });
            result.scripts.push(DiscoveredScript {
                name: "install".to_string(),
                command: "poetry install".to_string(),
                source: ScriptSource::Detected,
                category: ScriptCategory::Utility,
                description: Some("Install dependencies".to_string()),
                ports: vec![],
                env_required: vec![],
            });
        }
        PythonEnvType::Pipenv => {
            result.scripts.push(DiscoveredScript {
                name: "shell".to_string(),
                command: "pipenv shell".to_string(),
                source: ScriptSource::Detected,
                category: ScriptCategory::Dev,
                description: Some("Activate Pipenv environment".to_string()),
                ports: vec![],
                env_required: vec![],
            });
            result.scripts.push(DiscoveredScript {
                name: "install".to_string(),
                command: "pipenv install".to_string(),
                source: ScriptSource::Detected,
                category: ScriptCategory::Utility,
                description: Some("Install dependencies".to_string()),
                ports: vec![],
                env_required: vec![],
            });
        }
        PythonEnvType::Uv => {
            result.scripts.push(DiscoveredScript {
                name: "sync".to_string(),
                command: "uv sync".to_string(),
                source: ScriptSource::Detected,
                category: ScriptCategory::Utility,
                description: Some("Sync dependencies".to_string()),
                ports: vec![],
                env_required: vec![],
            });
        }
        PythonEnvType::Venv => {
            result.scripts.push(DiscoveredScript {
                name: "activate".to_string(),
                command: "source .venv/bin/activate".to_string(),
                source: ScriptSource::Detected,
                category: ScriptCategory::Dev,
                description: Some("Activate virtual environment".to_string()),
                ports: vec![],
                env_required: vec![],
            });
            result.scripts.push(DiscoveredScript {
                name: "install".to_string(),
                command: "pip install -r requirements.txt".to_string(),
                source: ScriptSource::Detected,
                category: ScriptCategory::Utility,
                description: Some("Install dependencies".to_string()),
                ports: vec![],
                env_required: vec![],
            });
        }
        _ => {}
    }

    // Add framework-specific commands
    match &framework {
        Some(PythonFramework::Django) => {
            result.scripts.push(DiscoveredScript {
                name: "runserver".to_string(),
                command: "python manage.py runserver".to_string(),
                source: ScriptSource::Detected,
                category: ScriptCategory::Dev,
                description: Some("Start Django development server".to_string()),
                ports: vec![8000],
                env_required: vec![],
            });
            result.scripts.push(DiscoveredScript {
                name: "migrate".to_string(),
                command: "python manage.py migrate".to_string(),
                source: ScriptSource::Detected,
                category: ScriptCategory::Database,
                description: Some("Run database migrations".to_string()),
                ports: vec![],
                env_required: vec![],
            });
            result.scripts.push(DiscoveredScript {
                name: "makemigrations".to_string(),
                command: "python manage.py makemigrations".to_string(),
                source: ScriptSource::Detected,
                category: ScriptCategory::Database,
                description: Some("Create new migrations".to_string()),
                ports: vec![],
                env_required: vec![],
            });
            result.expected_ports.push(ExpectedPort {
                port: 8000,
                source: "Django".to_string(),
                service_name: "django".to_string(),
            });
        }
        Some(PythonFramework::Flask) => {
            result.scripts.push(DiscoveredScript {
                name: "run".to_string(),
                command: "flask run".to_string(),
                source: ScriptSource::Detected,
                category: ScriptCategory::Dev,
                description: Some("Start Flask development server".to_string()),
                ports: vec![5000],
                env_required: vec!["FLASK_APP".to_string()],
            });
            result.expected_ports.push(ExpectedPort {
                port: 5000,
                source: "Flask".to_string(),
                service_name: "flask".to_string(),
            });
            result.env_vars.push(EnvVarSpec {
                name: "FLASK_APP".to_string(),
                description: Some("Flask application entry point".to_string()),
                source: "Flask".to_string(),
                example_value: Some("app.py".to_string()),
                is_secret: false,
            });
        }
        Some(PythonFramework::FastAPI) => {
            result.scripts.push(DiscoveredScript {
                name: "dev".to_string(),
                command: "uvicorn main:app --reload".to_string(),
                source: ScriptSource::Detected,
                category: ScriptCategory::Dev,
                description: Some("Start FastAPI development server".to_string()),
                ports: vec![8000],
                env_required: vec![],
            });
            result.expected_ports.push(ExpectedPort {
                port: 8000,
                source: "FastAPI".to_string(),
                service_name: "fastapi".to_string(),
            });
        }
        _ => {}
    }

    // Common Python commands
    result.scripts.push(DiscoveredScript {
        name: "test".to_string(),
        command: "pytest".to_string(),
        source: ScriptSource::Detected,
        category: ScriptCategory::Test,
        description: Some("Run tests with pytest".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    result.scripts.push(DiscoveredScript {
        name: "lint".to_string(),
        command: "ruff check .".to_string(),
        source: ScriptSource::Detected,
        category: ScriptCategory::Lint,
        description: Some("Run linter".to_string()),
        ports: vec![],
        env_required: vec![],
    });

    Ok(Some(result))
}

fn detect_env_type(root: &Path) -> PythonEnvType {
    if root.join("poetry.lock").exists() {
        PythonEnvType::Poetry
    } else if root.join("Pipfile.lock").exists() {
        PythonEnvType::Pipenv
    } else if root.join("uv.lock").exists() {
        PythonEnvType::Uv
    } else if root.join(".venv").exists() || root.join("venv").exists() {
        PythonEnvType::Venv
    } else if root.join("environment.yml").exists() {
        PythonEnvType::Conda
    } else {
        PythonEnvType::None
    }
}

async fn detect_framework(root: &Path) -> Result<Option<PythonFramework>> {
    // Check pyproject.toml
    if let Ok(content) = tokio::fs::read_to_string(root.join("pyproject.toml")).await {
        if content.contains("django") {
            return Ok(Some(PythonFramework::Django));
        }
        if content.contains("fastapi") {
            return Ok(Some(PythonFramework::FastAPI));
        }
        if content.contains("flask") {
            return Ok(Some(PythonFramework::Flask));
        }
        if content.contains("starlette") {
            return Ok(Some(PythonFramework::Starlette));
        }
    }

    // Check requirements.txt
    if let Ok(content) = tokio::fs::read_to_string(root.join("requirements.txt")).await {
        if content.to_lowercase().contains("django") {
            return Ok(Some(PythonFramework::Django));
        }
        if content.to_lowercase().contains("fastapi") {
            return Ok(Some(PythonFramework::FastAPI));
        }
        if content.to_lowercase().contains("flask") {
            return Ok(Some(PythonFramework::Flask));
        }
    }

    // Check for manage.py (Django)
    if root.join("manage.py").exists() {
        return Ok(Some(PythonFramework::Django));
    }

    Ok(None)
}
