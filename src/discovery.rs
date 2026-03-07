use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("failed to parse cargo metadata: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("failed to run cargo metadata: {0}")]
    CargoMetadataFailed(String),
    #[error("subprocess error: {0}")]
    Subprocess(#[from] crate::subprocess::SubprocessError),
}

#[derive(Debug)]
pub struct BinaryTarget {
    pub name: String,
    pub package_name: String,
    pub package_version: String,
    target_directory: PathBuf,
}

impl BinaryTarget {
    /// Path where `cargo build --release` places the binary.
    pub fn release_path(&self) -> PathBuf {
        self.target_directory.join("release").join(&self.name)
    }

    /// Path where cargo-sign places the signed binary.
    pub fn signed_release_path(&self) -> PathBuf {
        self.target_directory
            .join("signed")
            .join("release")
            .join(&self.name)
    }
}

/// Parse `cargo metadata --format-version 1 --no-deps` JSON output
/// and extract all binary targets.
pub fn parse_metadata(json: &str) -> Result<Vec<BinaryTarget>, DiscoveryError> {
    let meta: serde_json::Value = serde_json::from_str(json)?;

    let target_directory = meta["target_directory"]
        .as_str()
        .unwrap_or("target")
        .to_string();

    let packages = meta["packages"].as_array();
    let Some(packages) = packages else {
        return Ok(Vec::new());
    };

    let mut binaries = Vec::new();

    for pkg in packages {
        let pkg_name = pkg["name"].as_str().unwrap_or_default().to_string();
        let pkg_version = pkg["version"].as_str().unwrap_or_default().to_string();

        let Some(targets) = pkg["targets"].as_array() else {
            continue;
        };

        for target in targets {
            let kinds = target["kind"].as_array();
            let is_bin = kinds
                .is_some_and(|kinds| kinds.iter().any(|k| k.as_str().is_some_and(|s| s == "bin")));

            if is_bin {
                let name = target["name"].as_str().unwrap_or_default().to_string();
                binaries.push(BinaryTarget {
                    name,
                    package_name: pkg_name.clone(),
                    package_version: pkg_version.clone(),
                    target_directory: PathBuf::from(&target_directory),
                });
            }
        }
    }

    Ok(binaries)
}

/// Run `cargo metadata` and extract binary targets.
pub fn discover_binaries() -> Result<Vec<BinaryTarget>, DiscoveryError> {
    let output = crate::subprocess::run(
        "cargo",
        &["metadata", "--format-version", "1", "--no-deps"],
        false,
    )?;
    if !output.success {
        return Err(DiscoveryError::CargoMetadataFailed(output.stderr));
    }
    parse_metadata(&output.stdout)
}
