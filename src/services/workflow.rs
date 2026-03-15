use crate::config::Workflow;
use crate::rag;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{error, info, warn};

use super::executors::{ExecutorCapability, ExecutorRegistry};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepResult {
    pub name: String,
    pub success: bool,
    pub output: String,
    pub diagnosis: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunReport {
    pub workflow_name: String,
    pub steps: Vec<StepResult>,
    pub halted_on_failure: bool,
}

#[async_trait]
pub trait ErrorTriager: Send + Sync {
    async fn triage(&self, error_log: &str, model: &str) -> Result<String, String>;
}

pub struct OpenAiErrorTriager;

#[async_trait]
impl ErrorTriager for OpenAiErrorTriager {
    async fn triage(&self, error_log: &str, model: &str) -> Result<String, String> {
        rag::triage_error(error_log, model).await
    }
}

pub struct WorkflowService {
    executors: Arc<ExecutorRegistry>,
    triager: Arc<dyn ErrorTriager>,
    model: String,
}

impl WorkflowService {
    pub fn new(model: String, executors: Arc<ExecutorRegistry>) -> Self {
        Self {
            executors,
            triager: Arc::new(OpenAiErrorTriager),
            model,
        }
    }

    fn with_dependencies(
        model: String,
        executors: Arc<ExecutorRegistry>,
        triager: Arc<dyn ErrorTriager>,
    ) -> Self {
        Self {
            executors,
            triager,
            model,
        }
    }

    pub fn capabilities(&self) -> Vec<ExecutorCapability> {
        self.executors.capabilities()
    }

    pub async fn run_workflow(&self, workflow: Workflow) -> WorkflowRunReport {
        info!(workflow = %workflow.name, "Starting workflow execution");
        let mut results = Vec::new();

        for step in workflow.steps {
            let kind = step.action.kind();
            info!(step = %step.name, executor = %kind.as_str(), "Running workflow step");
            match self.executors.execute_step(&step).await {
                Ok(output) => {
                    info!(step = %step.name, executor = %kind.as_str(), "Workflow step succeeded");
                    results.push(StepResult {
                        name: step.name,
                        success: true,
                        output,
                        diagnosis: None,
                    });
                }
                Err(error_message) => {
                    warn!(step = %step.name, executor = %kind.as_str(), "Workflow step failed; requesting triage");
                    let diagnosis = match self.triager.triage(&error_message, &self.model).await {
                        Ok(diagnosis) => Some(diagnosis),
                        Err(err) => {
                            error!(step = %step.name, error = %err, "Failed to triage error");
                            None
                        }
                    };

                    results.push(StepResult {
                        name: step.name,
                        success: false,
                        output: error_message,
                        diagnosis,
                    });

                    return WorkflowRunReport {
                        workflow_name: workflow.name,
                        steps: results,
                        halted_on_failure: true,
                    };
                }
            }
        }

        WorkflowRunReport {
            workflow_name: workflow.name,
            steps: results,
            halted_on_failure: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ExecutorKind, Step, StepAction, Workflow};
    use crate::services::executors::ExecutorRegistry;
    use std::collections::HashSet;

    struct MockTriager {
        response: Result<String, String>,
    }

    #[async_trait]
    impl ErrorTriager for MockTriager {
        async fn triage(&self, _error_log: &str, _model: &str) -> Result<String, String> {
            self.response.clone()
        }
    }

    fn sample_workflow() -> Workflow {
        Workflow {
            name: "Test Workflow".to_string(),
            steps: vec![
                Step {
                    name: "step-1".to_string(),
                    action: StepAction::Shell {
                        command: "echo one".to_string(),
                    },
                },
                Step {
                    name: "step-2".to_string(),
                    action: StepAction::Shell {
                        command: "echo two".to_string(),
                    },
                },
            ],
        }
    }

    #[tokio::test]
    async fn runs_all_steps_when_successful() {
        let registry = Arc::new(ExecutorRegistry::new(HashSet::from([ExecutorKind::Shell])));
        let triager = Arc::new(MockTriager {
            response: Ok("unused".to_string()),
        });

        let service =
            WorkflowService::with_dependencies("test-model".to_string(), registry, triager);
        let report = service.run_workflow(sample_workflow()).await;

        assert!(!report.halted_on_failure);
        assert_eq!(report.steps.len(), 2);
        assert!(report.steps.iter().all(|s| s.success));
    }

    #[tokio::test]
    async fn stops_on_failure_and_records_diagnosis() {
        let registry = Arc::new(ExecutorRegistry::new(HashSet::new()));
        let triager = Arc::new(MockTriager {
            response: Ok("check permissions".to_string()),
        });

        let service =
            WorkflowService::with_dependencies("test-model".to_string(), registry, triager);
        let report = service.run_workflow(sample_workflow()).await;

        assert!(report.halted_on_failure);
        assert_eq!(report.steps.len(), 1);
        assert!(!report.steps[0].success);
        assert_eq!(
            report.steps[0].diagnosis.as_deref(),
            Some("check permissions")
        );
    }
}
