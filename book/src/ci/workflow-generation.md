# Workflow Generation

Generate a GitHub Actions workflow from your `sign.toml`:

```bash
cargo codesign workflow
```

This reads the configured platforms and env var names from `sign.toml` and generates `.github/workflows/release-sign.yml` with the correct secrets mappings.

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `--output <PATH>` | `.github/workflows/release-sign.yml` | Output path for the generated YAML |
| `--config <PATH>` | auto-discovered `sign.toml` | Path to sign.toml |

## What gets generated

For each platform configured in `sign.toml`, the workflow creates a job on the appropriate runner:

- **macOS** → `macos-latest`
- **Windows** → `windows-latest`
- **Linux** → `ubuntu-latest`

Each job:
1. Installs `cargo-codesign`
2. Runs `cargo codesign status` to verify credentials
3. Runs the platform-specific signing command

Secrets are mapped from the env var names in `sign.toml` to `${{ secrets.X }}`.

## Calling from another workflow

The generated workflow uses `workflow_call`, so you can invoke it from your release workflow:

```yaml
sign:
  needs: [build]
  uses: ./.github/workflows/release-sign.yml
  secrets: inherit
```
