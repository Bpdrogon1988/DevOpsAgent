use serde::{Deserialize, Serialize};
use std::{collections::HashSet, env, fs, net::SocketAddr, str::FromStr};

#[derive(Debug, Deserialize, Clone)]
pub struct Workflow {
    pub name: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Step {
    pub name: String,
    #[serde(flatten)]
    pub action: StepAction,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StepAction {
    Shell {
        command: String,
    },
    Docker {
        command: String,
    },
    Kubernetes {
        command: String,
    },
    Terraform {
        command: String,
    },
    Ansible {
        command: String,
    },
    Http {
        url: String,
        #[serde(default = "default_http_method")]
        method: String,
        body: Option<String>,
    },
}

fn default_http_method() -> String {
    "GET".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutorKind {
    Shell,
    Docker,
    Kubernetes,
    Terraform,
    Ansible,
    Http,
}

impl ExecutorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutorKind::Shell => "shell",
            ExecutorKind::Docker => "docker",
            ExecutorKind::Kubernetes => "kubernetes",
            ExecutorKind::Terraform => "terraform",
            ExecutorKind::Ansible => "ansible",
            ExecutorKind::Http => "http",
        }
    }

    pub fn all() -> [ExecutorKind; 6] {
        [
            ExecutorKind::Shell,
            ExecutorKind::Docker,
            ExecutorKind::Kubernetes,
            ExecutorKind::Terraform,
            ExecutorKind::Ansible,
            ExecutorKind::Http,
        ]
    }
}

impl FromStr for ExecutorKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "shell" => Ok(ExecutorKind::Shell),
            "docker" => Ok(ExecutorKind::Docker),
            "kubernetes" | "kubectl" | "k8s" => Ok(ExecutorKind::Kubernetes),
            "terraform" => Ok(ExecutorKind::Terraform),
            "ansible" => Ok(ExecutorKind::Ansible),
            "http" => Ok(ExecutorKind::Http),
            _ => Err(format!("Unsupported executor type: {value}")),
        }
    }
}

impl StepAction {
    pub fn kind(&self) -> ExecutorKind {
        match self {
            StepAction::Shell { .. } => ExecutorKind::Shell,
            StepAction::Docker { .. } => ExecutorKind::Docker,
            StepAction::Kubernetes { .. } => ExecutorKind::Kubernetes,
            StepAction::Terraform { .. } => ExecutorKind::Terraform,
            StepAction::Ansible { .. } => ExecutorKind::Ansible,
            StepAction::Http { .. } => ExecutorKind::Http,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub bind_addr: SocketAddr,
    pub github_webhook_secret: String,
    pub openai_model: String,
    pub enabled_executors: HashSet<ExecutorKind>,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub runtime: RuntimeConfig,
    pub workflow: Workflow,
}

pub fn load_config() -> Result<AppConfig, String> {
    dotenv::dotenv().ok();

    let github_webhook_secret = env::var("GITHUB_WEBHOOK_SECRET")
        .map_err(|_| "GITHUB_WEBHOOK_SECRET is required".to_string())?;

    let bind_addr = env::var("BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_string())
        .parse::<SocketAddr>()
        .map_err(|err| format!("Invalid BIND_ADDR: {err}"))?;

    let openai_model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

    let enabled_executors = parse_enabled_executors(env::var("ENABLED_EXECUTORS").ok())?;

    let workflow = load_workflow_file("workflow.yaml")?;
    validate_workflow_executors(&workflow, &enabled_executors)?;

    Ok(AppConfig {
        runtime: RuntimeConfig {
            bind_addr,
            github_webhook_secret,
            openai_model,
            enabled_executors,
        },
        workflow,
    })
}

fn parse_enabled_executors(config_value: Option<String>) -> Result<HashSet<ExecutorKind>, String> {
    let raw = config_value.unwrap_or_else(|| "shell,http".to_string());
    let mut parsed = HashSet::new();

    for token in raw.split(',') {
        if token.trim().is_empty() {
            continue;
        }
        parsed.insert(ExecutorKind::from_str(token)?);
    }

    if parsed.is_empty() {
        return Err("ENABLED_EXECUTORS must contain at least one executor".to_string());
    }

    Ok(parsed)
}

fn validate_workflow_executors(
    workflow: &Workflow,
    enabled_executors: &HashSet<ExecutorKind>,
) -> Result<(), String> {
    for step in &workflow.steps {
        let kind = step.action.kind();
        if !enabled_executors.contains(&kind) {
            return Err(format!(
                "Step '{}' uses disabled executor '{}'. Enable it in ENABLED_EXECUTORS.",
                step.name,
                kind.as_str()
            ));
        }
    }
    Ok(())
}

fn load_workflow_file(path: &str) -> Result<Workflow, String> {
    let yaml_str =
        fs::read_to_string(path).map_err(|err| format!("Failed to read {path}: {err}"))?;
    parse_workflow_from_str(&yaml_str)
}

pub fn parse_workflow_from_str(yaml_str: &str) -> Result<Workflow, String> {
    serde_yaml::from_str(yaml_str).map_err(|err| format!("Failed to parse workflow yaml: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_workflow() {
        let yaml = r#"
name: "Deploy"
steps:
  - name: "Build"
    type: "shell"
    command: "cargo build"
"#;

        let workflow = parse_workflow_from_str(yaml).expect("expected workflow to parse");
        assert_eq!(workflow.name, "Deploy");
        assert_eq!(workflow.steps.len(), 1);
        assert_eq!(workflow.steps[0].name, "Build");
        assert_eq!(workflow.steps[0].action.kind(), ExecutorKind::Shell);
    }

    #[test]
    fn parse_invalid_workflow() {
        let yaml = r#"
name: "Deploy"
steps:
  - name: "Build"
"#;

        assert!(parse_workflow_from_str(yaml).is_err());
    }

    #[test]
    fn parse_enabled_executors_rejects_unknown() {
        let err = parse_enabled_executors(Some("shell,unknown".to_string())).unwrap_err();
        assert!(err.contains("Unsupported executor type"));
    }

    #[test]
    fn validate_workflow_rejects_disabled_executor() {
        let workflow = parse_workflow_from_str(
            r#"
name: "Deploy"
steps:
  - name: "Terraform plan"
    type: "terraform"
    command: "plan"
"#,
        )
        .expect("parse workflow");

        let enabled = HashSet::from([ExecutorKind::Shell]);
        let err = validate_workflow_executors(&workflow, &enabled).unwrap_err();
        assert!(err.contains("disabled executor 'terraform'"));
    }
}
