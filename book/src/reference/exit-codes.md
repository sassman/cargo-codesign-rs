# Exit Codes

| Code | Meaning | Example |
|------|---------|---------|
| `0` | Success | Signing completed, all checks passed |
| `1` | Signing failed | Credential rejected, tool error, Apple/Azure rejection |
| `2` | Configuration error | Missing `sign.toml`, bad TOML syntax, missing required section |
| `3` | Prerequisite missing | `codesign` not found, wrong platform (e.g., `cargo codesign macos` on Linux) |
| `4` | Validation failed | `cargo codesign status` found one or more failing checks |

## Usage in CI

```yaml
- name: Check signing setup
  run: cargo codesign status
  # Exits 4 if any check fails — CI step fails

- name: Sign
  run: cargo codesign macos --app "MyApp.app"
  # Exits 1 on signing failure
  # Exits 2 if sign.toml is broken
```

## Distinguishing errors

Exit code `2` (config error) means the problem is in your `sign.toml` or CLI flags — fix the config. Exit code `1` (signing failed) means the config is valid but the operation failed — check credentials, network, or Apple's status page.
