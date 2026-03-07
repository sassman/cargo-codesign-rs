use super::SignConfig;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("no sign.toml found (searched ./sign.toml and ./.cargo/sign.toml)")]
    NotFound,
    #[error("failed to read {path}: {source}")]
    ReadError {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse {path}: {source}")]
    ParseError {
        path: PathBuf,
        source: toml::de::Error,
    },
}

pub fn resolve_config_from_path(
    path: &Path,
) -> Result<(SignConfig, PathBuf, Vec<String>), ResolveError> {
    let content = std::fs::read_to_string(path).map_err(|e| ResolveError::ReadError {
        path: path.to_path_buf(),
        source: e,
    })?;
    let config: SignConfig = toml::from_str(&content).map_err(|e| ResolveError::ParseError {
        path: path.to_path_buf(),
        source: e,
    })?;
    Ok((config, path.to_path_buf(), Vec::new()))
}

pub fn resolve_config(
    working_dir: Option<&Path>,
) -> Result<(SignConfig, PathBuf, Vec<String>), ResolveError> {
    let cwd = match working_dir {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir().expect("cannot determine current directory"),
    };

    let root_path = cwd.join("sign.toml");
    let cargo_path = cwd.join(".cargo").join("sign.toml");

    let root_exists = root_path.exists();
    let cargo_exists = cargo_path.exists();

    match (root_exists, cargo_exists) {
        (true, true) => {
            let (config, path, _) = resolve_config_from_path(&root_path)?;
            let warnings = vec![
                "warning: both ./sign.toml and ./.cargo/sign.toml exist, using ./sign.toml"
                    .to_string(),
            ];
            Ok((config, path, warnings))
        }
        (true, false) => resolve_config_from_path(&root_path),
        (false, true) => resolve_config_from_path(&cargo_path),
        (false, false) => Err(ResolveError::NotFound),
    }
}
