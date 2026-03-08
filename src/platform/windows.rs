use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum WindowsSignError {
    #[error("subprocess failed: {0}")]
    Subprocess(#[from] crate::subprocess::SubprocessError),
    #[error("signing failed for {path}: {detail}")]
    SigningFailed { path: PathBuf, detail: String },
    #[error("verification failed for {path}: {detail}")]
    VerificationFailed { path: PathBuf, detail: String },
    #[error("tool not found: {0}")]
    ToolNotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct SignOpts<'a> {
    pub endpoint: &'a str,
    pub account_name: &'a str,
    pub cert_profile: &'a str,
    pub timestamp_server: &'a str,
    pub dlib_path: &'a Path,
    pub verbose: bool,
}

/// Generate the `metadata.json` file required by Azure Trusted Signing.
pub fn generate_metadata_json(endpoint: &str, account_name: &str, cert_profile: &str) -> String {
    serde_json::json!({
        "Endpoint": endpoint,
        "CodeSigningAccountName": account_name,
        "CertificateProfileName": cert_profile,
    })
    .to_string()
}

/// Sign a Windows executable using `signtool.exe` with Azure Trusted Signing.
#[cfg(target_os = "windows")]
pub fn sign_exe(exe_path: &Path, opts: &SignOpts<'_>) -> Result<(), WindowsSignError> {
    use crate::subprocess::run;

    let exe_str = exe_path.to_string_lossy().to_string();
    let dlib_str = opts.dlib_path.to_string_lossy().to_string();

    let metadata = generate_metadata_json(opts.endpoint, opts.account_name, opts.cert_profile);
    let metadata_path = exe_path.with_extension("metadata.json");
    std::fs::write(&metadata_path, &metadata)?;
    let metadata_str = metadata_path.to_string_lossy().to_string();

    let output = run(
        "signtool",
        &[
            "sign",
            "/fd",
            "SHA256",
            "/tr",
            opts.timestamp_server,
            "/td",
            "SHA256",
            "/dlib",
            &dlib_str,
            "/dmdf",
            &metadata_str,
            &exe_str,
        ],
        opts.verbose,
    )?;

    // Clean up metadata file
    let _ = std::fs::remove_file(&metadata_path);

    if !output.success {
        return Err(WindowsSignError::SigningFailed {
            path: exe_path.to_path_buf(),
            detail: output.stderr,
        });
    }
    Ok(())
}

/// Verify a signed Windows executable.
#[cfg(target_os = "windows")]
pub fn verify_exe(exe_path: &Path, verbose: bool) -> Result<(), WindowsSignError> {
    use crate::subprocess::run;

    let exe_str = exe_path.to_string_lossy().to_string();
    let output = run("signtool", &["verify", "/pa", "/v", &exe_str], verbose)?;
    if !output.success {
        return Err(WindowsSignError::VerificationFailed {
            path: exe_path.to_path_buf(),
            detail: output.stderr,
        });
    }
    Ok(())
}

/// Install Azure Trusted Signing tools via nuget.
#[cfg(target_os = "windows")]
pub fn install_tools(verbose: bool) -> Result<PathBuf, WindowsSignError> {
    use crate::subprocess::run;

    let output = run(
        "nuget",
        &[
            "install",
            "Microsoft.Trusted.Signing.Client",
            "-OutputDirectory",
            ".signing-tools",
        ],
        verbose,
    )?;
    if !output.success {
        return Err(WindowsSignError::ToolNotFound(
            "nuget install Microsoft.Trusted.Signing.Client failed".to_string(),
        ));
    }

    // Find the DLL in the installed package
    let tools_dir = PathBuf::from(".signing-tools");
    for entry in std::fs::read_dir(&tools_dir)? {
        let entry = entry?;
        let dll_path = entry
            .path()
            .join("bin")
            .join("x64")
            .join("Azure.CodeSigning.Dlib.dll");
        if dll_path.exists() {
            return Ok(dll_path);
        }
    }

    Err(WindowsSignError::ToolNotFound(
        "Azure.CodeSigning.Dlib.dll not found after install".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_json_has_expected_fields() {
        let json = generate_metadata_json(
            "https://wus2.codesigning.azure.net",
            "my-account",
            "my-profile",
        );
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["Endpoint"], "https://wus2.codesigning.azure.net");
        assert_eq!(parsed["CodeSigningAccountName"], "my-account");
        assert_eq!(parsed["CertificateProfileName"], "my-profile");
    }

    #[test]
    fn metadata_json_is_valid_json() {
        let json = generate_metadata_json("e", "a", "p");
        assert!(serde_json::from_str::<serde_json::Value>(&json).is_ok());
    }
}
