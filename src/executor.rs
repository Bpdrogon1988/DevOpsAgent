use std::process::Command;
use tokio::task;

pub async fn run_script(script: String) -> Result<String, String> {
    let output = task::spawn_blocking(move || Command::new("sh").arg("-c").arg(script).output())
        .await
        .map_err(|e| e.to_string())?;

    command_output_to_result(output)
}

pub async fn run_program(binary: String, args: Vec<String>) -> Result<String, String> {
    let output = task::spawn_blocking(move || Command::new(binary).args(args).output())
        .await
        .map_err(|e| e.to_string())?;

    command_output_to_result(output)
}

fn command_output_to_result(
    output: std::io::Result<std::process::Output>,
) -> Result<String, String> {
    match output {
        Ok(out) => {
            if out.status.success() {
                Ok(String::from_utf8_lossy(&out.stdout).to_string())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).to_string())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}
