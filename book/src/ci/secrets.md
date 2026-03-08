# Secrets Management

cargo-codesign follows a strict separation: **`sign.toml` stores env var names, never values.** All secret values reach cargo-codesign exclusively through environment variables.

## Local development

Use a `.env` file in your project root. cargo-codesign loads it automatically via [dotenvy](https://crates.io/crates/dotenvy):

```bash
# .env — NEVER commit this
APPLE_ID=you@example.com
APPLE_TEAM_ID=ABCDE12345
APPLE_APP_PASSWORD=xxxx-xxxx-xxxx-xxxx
```

Make sure `.env` is in your `.gitignore`.

## CI (GitHub Actions)

Store secrets in your repository settings: **Settings → Secrets and variables → Actions**.

Map them to env vars in your workflow:

```yaml
env:
  APPLE_ID: ${{ secrets.APPLE_ID }}
  APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
  APPLE_APP_PASSWORD: ${{ secrets.APPLE_APP_PASSWORD }}
```

## The sign.toml contract

```toml
[macos.env]
apple-id = "APPLE_ID"       # ← this is the env var NAME, not the value
team-id = "APPLE_TEAM_ID"
app-password = "APPLE_APP_PASSWORD"
```

If `sign.toml` contains what looks like an actual secret value instead of an env var name, cargo-codesign will reject it.

## Rotation

- **apple-id mode**: Revoke and regenerate the app-specific password at [account.apple.com](https://account.apple.com). Update the `APPLE_APP_PASSWORD` secret.
- **api-key mode**: Revoke the key in App Store Connect and create a new one. Update the `APPLE_NOTARIZATION_KEY`, `APPLE_NOTARIZATION_KEY_ID` secrets. (Issuer ID doesn't change.)
- **Update signing keys**: Generate a new keypair with `cargo codesign keygen`, update `UPDATE_SIGNING_KEY` in CI, and ship a new binary with the updated public key. Old signatures won't verify against the new key — this is intentional.
