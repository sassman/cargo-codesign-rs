/// Determine the default signature path for a given method.
pub fn default_signature_path(artifact: &std::path::Path, method: &str) -> std::path::PathBuf {
    let ext = match method {
        "cosign" => ".bundle",
        "minisign" => ".minisig",
        _ => ".sig",
    };
    let mut p = artifact.as_os_str().to_owned();
    p.push(ext);
    std::path::PathBuf::from(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn default_sig_path_update() {
        assert_eq!(
            default_signature_path(Path::new("release.tar.gz"), "update"),
            PathBuf::from("release.tar.gz.sig")
        );
    }

    #[test]
    fn default_sig_path_cosign() {
        assert_eq!(
            default_signature_path(Path::new("release.tar.gz"), "cosign"),
            PathBuf::from("release.tar.gz.bundle")
        );
    }

    #[test]
    fn default_sig_path_minisign() {
        assert_eq!(
            default_signature_path(Path::new("release.tar.gz"), "minisign"),
            PathBuf::from("release.tar.gz.minisig")
        );
    }

    #[test]
    fn default_sig_path_gpg() {
        assert_eq!(
            default_signature_path(Path::new("release.tar.gz"), "gpg"),
            PathBuf::from("release.tar.gz.sig")
        );
    }
}
