use crate::{
    config::{ExecutorKind, Step, StepAction},
    executor,
};
use async_trait::async_trait;
use reqwest::{Client, Method};
use serde::Serialize;
use std::{collections::HashMap, collections::HashSet, sync::Arc, time::Duration};

#[derive(Debug, Clone, Serialize)]
pub struct ExecutorCapability {
    pub kind: ExecutorKind,
    pub enabled: bool,
    pub description: &'static str,
    pub risk_level: &'static str,
}

#[async_trait]
pub trait StepExecutor: Send + Sync {
    fn kind(&self) -> ExecutorKind;
    fn description(&self) -> &'static str;
    fn risk_level(&self) -> &'static str;
    async fn execute(&self, action: &StepAction) -> Result<String, String>;
}

pub struct ExecutorRegistry {
    executors: HashMap<ExecutorKind, Arc<dyn StepExecutor>>,
    enabled: HashSet<ExecutorKind>,
}

impl ExecutorRegistry {
    pub fn new(enabled: HashSet<ExecutorKind>) -> Self {
        let mut executors: HashMap<ExecutorKind, Arc<dyn StepExecutor>> = HashMap::new();

        let builtins: Vec<Arc<dyn StepExecutor>> = vec![
            Arc::new(ShellExecutor),
            Arc::new(CliWrapperExecutor::new(
                ExecutorKind::Docker,
                "docker",
                "Docker CLI executor",
                "high",
            )),
            Arc::new(CliWrapperExecutor::new(
                ExecutorKind::Kubernetes,
                "kubectl",
                "Kubernetes kubectl executor",
                "high",
            )),
            Arc::new(CliWrapperExecutor::new(
                ExecutorKind::Terraform,
                "terraform",
                "Terraform CLI executor",
                "high",
            )),
            Arc::new(CliWrapperExecutor::new(
                ExecutorKind::Ansible,
                "ansible-playbook",
                "Ansible playbook executor",
                "high",
            )),
            Arc::new(HttpExecutor::new()),
        ];

        for executor in builtins {
            executors.insert(executor.kind(), executor);
        }

        Self { executors, enabled }
    }

    pub async fn execute_step(&self, step: &Step) -> Result<String, String> {
        let kind = step.action.kind();
        if !self.enabled.contains(&kind) {
            return Err(format!(
                "Executor '{}' is disabled by policy",
                kind.as_str()
            ));
        }

        let Some(executor) = self.executors.get(&kind) else {
            return Err(format!("Executor '{}' is not registered", kind.as_str()));
        };

        executor.execute(&step.action).await
    }

    pub fn capabilities(&self) -> Vec<ExecutorCapability> {
        let mut capabilities = Vec::new();
        for kind in ExecutorKind::all() {
            if let Some(executor) = self.executors.get(&kind) {
                capabilities.push(ExecutorCapability {
                    kind,
                    enabled: self.enabled.contains(&kind),
                    description: executor.description(),
                    risk_level: executor.risk_level(),
                });
            }
        }
        capabilities
    }
}

pub struct ShellExecutor;

#[async_trait]
impl StepExecutor for ShellExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::Shell
    }

    fn description(&self) -> &'static str {
        "Shell command executor"
    }

    fn risk_level(&self) -> &'static str {
        "high"
    }

    async fn execute(&self, action: &StepAction) -> Result<String, String> {
        match action {
            StepAction::Shell { command } => executor::run_script(command.clone()).await,
            _ => Err("ShellExecutor received incompatible action".to_string()),
        }
    }
}

pub struct CliWrapperExecutor {
    kind: ExecutorKind,
    binary: &'static str,
    description: &'static str,
    risk_level: &'static str,
}

impl CliWrapperExecutor {
    pub fn new(
        kind: ExecutorKind,
        binary: &'static str,
        description: &'static str,
        risk_level: &'static str,
    ) -> Self {
        Self {
            kind,
            binary,
            description,
            risk_level,
        }
    }
}

#[async_trait]
impl StepExecutor for CliWrapperExecutor {
    fn kind(&self) -> ExecutorKind {
        self.kind
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn risk_level(&self) -> &'static str {
        self.risk_level
    }

    async fn execute(&self, action: &StepAction) -> Result<String, String> {
        let raw = match action {
            StepAction::Docker { command }
            | StepAction::Kubernetes { command }
            | StepAction::Terraform { command }
            | StepAction::Ansible { command } => command,
            _ => return Err("CliWrapperExecutor received incompatible action".to_string()),
        };

        let args =
            shlex::split(raw).ok_or_else(|| "Failed to parse command arguments".to_string())?;
        executor::run_program(self.binary.to_string(), args).await
    }
}

pub struct HttpExecutor {
    client: Client,
}

impl HttpExecutor {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .expect("http client init"),
        }
    }
}

#[async_trait]
impl StepExecutor for HttpExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::Http
    }

    fn description(&self) -> &'static str {
        "Outbound HTTP executor"
    }

    fn risk_level(&self) -> &'static str {
        "medium"
    }

    async fn execute(&self, action: &StepAction) -> Result<String, String> {
        let (url, method, body) = match action {
            StepAction::Http { url, method, body } => (url, method, body),
            _ => return Err("HttpExecutor received incompatible action".to_string()),
        };

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err("HTTP executor only supports http:// or https:// URLs".to_string());
        }

        let parsed_method = Method::from_bytes(method.as_bytes())
            .map_err(|_| format!("Unsupported HTTP method: {method}"))?;

        let mut request = self.client.request(parsed_method, url);
        if let Some(payload) = body {
            request = request.body(payload.clone());
        }

        let response = request.send().await.map_err(|err| err.to_string())?;
        let status = response.status();
        let text = response.text().await.map_err(|err| err.to_string())?;

        if status.is_success() {
            Ok(text)
        } else {
            Err(format!("HTTP request failed with status {status}: {text}"))
        }
    }
}
