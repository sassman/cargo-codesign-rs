use std::fmt;
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

/// A command-line argument that may or may not contain sensitive data.
///
/// `Display` always redacts `Sensitive` values, so accidental logging or
/// formatting cannot leak secrets.
#[derive(Debug, Clone, Copy)]
pub enum Arg<'a> {
    Plain(&'a str),
    Sensitive(&'a str),
}

impl<'a> Arg<'a> {
    pub fn sensitive(s: &'a str) -> Self {
        Arg::Sensitive(s)
    }

    pub fn as_str(&self) -> &'a str {
        match self {
            Arg::Plain(s) | Arg::Sensitive(s) => s,
        }
    }
}

impl<'a> From<&'a str> for Arg<'a> {
    fn from(s: &'a str) -> Self {
        Arg::Plain(s)
    }
}

impl fmt::Display for Arg<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arg::Plain(s) => f.write_str(s),
            Arg::Sensitive(_) => f.write_str("****"),
        }
    }
}

/// Run a subprocess, passing all arguments as plain (non-sensitive) strings.
pub fn run(command: &str, args: &[&str], verbose: bool) -> Result<RunOutput, SubprocessError> {
    let typed: Vec<Arg<'_>> = args.iter().map(|&s| Arg::Plain(s)).collect();
    run_args(command, &typed, verbose)
}

/// Run a subprocess with typed arguments that distinguish sensitive values.
pub fn run_args(
    command: &str,
    args: &[Arg<'_>],
    verbose: bool,
) -> Result<RunOutput, SubprocessError> {
    if verbose {
        let display: Vec<_> = args.iter().map(ToString::to_string).collect();
        eprintln!("  $ {} {}", command, display.join(" "));
    }

    let raw: Vec<&str> = args.iter().map(Arg::as_str).collect();
    let output =
        Command::new(command)
            .args(&raw)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_arg_display_is_redacted() {
        let arg = Arg::sensitive("super-secret-password");
        assert_eq!(arg.to_string(), "****");
    }

    #[test]
    fn plain_arg_display_shows_value() {
        let arg = Arg::Plain("visible-value");
        assert_eq!(arg.to_string(), "visible-value");
    }

    #[test]
    fn sensitive_arg_as_str_exposes_real_value() {
        let arg = Arg::sensitive("real-password");
        assert_eq!(arg.as_str(), "real-password");
    }

    #[test]
    fn from_str_creates_plain_arg() {
        let arg: Arg<'_> = "hello".into();
        assert_eq!(arg.to_string(), "hello");
        assert_eq!(arg.as_str(), "hello");
    }

    #[test]
    fn mixed_args_display_redacts_only_sensitive() {
        let args: Vec<Arg<'_>> = vec![
            "create-keychain".into(),
            "-p".into(),
            Arg::sensitive("my-secret"),
            "keychain-name".into(),
        ];
        let display: Vec<String> = args.iter().map(ToString::to_string).collect();
        assert_eq!(display, &["create-keychain", "-p", "****", "keychain-name"]);
    }
}
