#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("failed to parse dist-manifest.json: {0}")]
    ParseError(#[from] serde_json::Error),
}

#[derive(Debug)]
pub struct DistArtifact {
    pub name: String,
    pub kind: String,
    pub target_triples: Vec<String>,
    pub asset_paths: Vec<String>,
}

impl DistArtifact {
    pub fn is_macos(&self) -> bool {
        self.target_triples
            .iter()
            .any(|t| t.contains("apple-darwin"))
    }

    pub fn is_windows(&self) -> bool {
        self.target_triples.iter().any(|t| t.contains("windows"))
    }

    pub fn is_linux(&self) -> bool {
        self.target_triples.iter().any(|t| t.contains("linux"))
    }
}

/// Parse cargo-dist's `dist-manifest.json` and extract all artifacts.
pub fn parse_dist_manifest(json: &str) -> Result<Vec<DistArtifact>, ManifestError> {
    let manifest: serde_json::Value = serde_json::from_str(json)?;

    let Some(artifacts) = manifest["artifacts"].as_object() else {
        return Ok(Vec::new());
    };

    let mut result = Vec::new();

    for (_key, artifact) in artifacts {
        let name = artifact["name"].as_str().unwrap_or_default().to_string();
        let kind = artifact["kind"].as_str().unwrap_or_default().to_string();

        let target_triples = artifact["target_triples"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let asset_paths = artifact["assets"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v["path"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        result.push(DistArtifact {
            name,
            kind,
            target_triples,
            asset_paths,
        });
    }

    Ok(result)
}
