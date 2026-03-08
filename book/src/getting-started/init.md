# Creating sign.toml

Run `cargo codesign init` to generate a `sign.toml` with guided prompts:

```bash
cargo codesign init
```

The wizard asks:

1. **Which platforms?** — macOS, Windows, Linux, Update signing
2. **Auth mode** (macOS) — `apple-id` for local/indie, `api-key` for CI
3. **Signing method** (Linux) — cosign, minisign, or gpg

After generating the file, it checks which credentials are already set in your environment and shows how to obtain any that are missing — with links to the relevant guide.

## Example output

```text
✓ Created sign.toml

Credential status (2 missing):
  ✓ APPLE_ID                            set
  ✗ APPLE_TEAM_ID                       Team ID from App Store Connect > Membership
    → https://sassman.github.io/cargo-codesign-rs/macos/credentials.html
  ✗ APPLE_APP_PASSWORD                  app-specific password for notarization
    → https://sassman.github.io/cargo-codesign-rs/macos/auth-modes.html

Set missing credentials in .env or CI secrets, then run:
  cargo codesign status
```

## Manual creation

You can also create `sign.toml` by hand — see the [sign.toml Reference](../reference/sign-toml.md) for the full format.
