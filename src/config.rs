use serde::Deserialize;
use std::fs;

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

pub fn load_config() -> Workflow {
    dotenv::dotenv().ok();
    println!("Environment variables loaded.");

    let yaml_str = fs::read_to_string("workflow.yaml")
        .expect("Failed to read workflow.yaml check if the file exists.");
    
    let workflow: Workflow = serde_yaml::from_str(&yaml_str)
        .expect("Failed to parse workflow.yaml syntax.");
        
    println!("Loaded workflow: {}", workflow.name);
    workflow
}
