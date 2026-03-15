use serde::Deserialize;
use std::{env, fs, net::SocketAddr};

#[derive(Debug, Deserialize, Clone)]
pub struct Workflow {
    pub name: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Step {
    pub name: String,
    pub command: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub bind_addr: SocketAddr,
    pub github_webhook_secret: String,
    pub openai_model: String,
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

    let workflow = load_workflow_file("workflow.yaml")?;

    Ok(AppConfig {
        runtime: RuntimeConfig {
            bind_addr,
            github_webhook_secret,
            openai_model,
        },
        workflow,
    })
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
    command: "cargo build"
"#;

        let workflow = parse_workflow_from_str(yaml).expect("expected workflow to parse");
        assert_eq!(workflow.name, "Deploy");
        assert_eq!(workflow.steps.len(), 1);
        assert_eq!(workflow.steps[0].name, "Build");
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
}
