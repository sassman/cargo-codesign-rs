# Workflow Generation

> This command is not yet implemented. For now, write your GitHub Actions workflow manually — see the [GitHub Actions Walkthrough](./github-actions.md) for a complete example.

`cargo codesign workflow` will generate GitHub Actions YAML that wires your `sign.toml` configuration directly into CI:

```bash
cargo codesign workflow --ci github-actions --output .github/workflows/release-sign.yml
```

It reads env var names from `sign.toml` to automatically generate `${{ secrets.X }}` mappings, so you don't need to manually maintain the YAML.
