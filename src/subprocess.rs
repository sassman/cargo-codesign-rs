use std::process::Command;

#[derive(Debug)]
pub struct RunOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
    pub code: Option<i32>,
}

#[derive(Debug, thiserror::Error)]
pub enum SubprocessError {
    #[error("failed to execute {command}: {source}")]
    SpawnFailed {
        command: String,
        source: std::io::Error,
    },
}

pub fn run(command: &str, args: &[&str], verbose: bool) -> Result<RunOutput, SubprocessError> {
    if verbose {
        eprintln!("  $ {} {}", command, args.join(" "));
    }

    let output =
        Command::new(command)
            .args(args)
            .output()
            .map_err(|e| SubprocessError::SpawnFailed {
                command: command.to_string(),
                source: e,
            })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if verbose && !stdout.is_empty() {
        eprint!("{stdout}");
    }
    if verbose && !stderr.is_empty() {
        eprint!("{stderr}");
    }

    Ok(RunOutput {
        stdout,
        stderr,
        success: output.status.success(),
        code: output.status.code(),
    })
}
