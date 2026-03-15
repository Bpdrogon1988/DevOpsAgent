use std::process::Command;
use tokio::task;

pub async fn run_script(script: String) -> Result<String, String> {
    // We execute the script in a background thread to prevent blocking the Tokio runtime
    let output = task::spawn_blocking(move || {
        Command::new("sh") // Adjust for windows e.g. "cmd" /C
            .arg("-c")
            .arg(script)
            .output()
    })
    .await
    .map_err(|e| e.to_string())?;

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
