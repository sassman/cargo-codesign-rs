# Creating sign.toml

> This command is not yet implemented. For now, create `sign.toml` manually — see the [sign.toml Reference](../reference/sign-toml.md) for the full format.

`cargo codesign init` will generate a `sign.toml` in the current directory with guided prompts:

- Which platforms do you target? (macOS, Windows, Linux)
- Which macOS auth mode? (api-key for CI, apple-id for local/indie)
- Which environment variable names for your secrets?

The generated file is a starting point — edit it to match your setup.

In the meantime, the quickest way to get started:

```bash
cat > sign.toml << 'EOF'
[macos]
identity = "Developer ID Application"
entitlements = "entitlements.plist"
auth = "apple-id"

[macos.env]
apple-id = "APPLE_ID"
team-id = "APPLE_TEAM_ID"
app-password = "APPLE_APP_PASSWORD"
EOF
```

Then check your setup with `cargo codesign status`.
