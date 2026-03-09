# Auth Modes: api-key vs apple-id

cargo-codesign supports two authentication modes for macOS notarization. The auth mode is configured in `sign.toml` and determines which credentials cargo-codesign expects.

## apple-id mode

Best for local development and indie developers.

```toml
[macos]
identity = "Developer ID Application"
entitlements = "entitlements.plist"
auth = "apple-id"

[macos.env]
apple-id = "APPLE_ID"
team-id = "APPLE_TEAM_ID"
app-password = "APPLE_APP_PASSWORD"
```

Under the hood, cargo-codesign calls:

```bash
xcrun notarytool submit artifact.dmg \
  --apple-id "$APPLE_ID" \
  --team-id "$APPLE_TEAM_ID" \
  --password "$APPLE_APP_PASSWORD" \
  --wait
```

**Credentials needed:**
- `APPLE_ID` — your Apple ID email
- `APPLE_TEAM_ID` — your 10-character team ID
- `APPLE_APP_PASSWORD` — an app-specific password (not your Apple ID password)

## api-key mode

Best for CI and team environments. Uses an App Store Connect API key (`.p8` file).

```toml
[macos]
identity = "Developer ID Application"
entitlements = "entitlements.plist"
auth = "api-key"

[macos.env]
certificate = "MACOS_CERTIFICATE"
certificate-password = "MACOS_CERTIFICATE_PASSWORD"
notarization-key = "APPLE_NOTARIZATION_KEY"
notarization-key-id = "APPLE_NOTARIZATION_KEY_ID"
notarization-issuer = "APPLE_NOTARIZATION_ISSUER_ID"
```

Under the hood, cargo-codesign calls:

```bash
xcrun notarytool submit artifact.dmg \
  --key "/tmp/AuthKey.p8" \
  --key-id "$APPLE_NOTARIZATION_KEY_ID" \
  --issuer "$APPLE_NOTARIZATION_ISSUER_ID" \
  --wait
```

The `APPLE_NOTARIZATION_KEY` env var contains the `.p8` file contents base64-encoded. cargo-codesign decodes it to a temp file, uses it, and deletes it.

**Credentials needed:**
- `MACOS_CERTIFICATE` — base64-encoded `.p12` certificate (for CI keychain import)
- `MACOS_CERTIFICATE_PASSWORD` — password for the `.p12`
- `APPLE_NOTARIZATION_KEY` — base64-encoded `.p8` API key
- `APPLE_NOTARIZATION_KEY_ID` — the key ID from App Store Connect
- `APPLE_NOTARIZATION_ISSUER_ID` — the issuer ID from App Store Connect

## Which mode should I use?

| Scenario | Recommended mode |
|----------|-----------------|
| Local dev on your Mac | `apple-id` |
| Solo developer, simple CI | `apple-id` |
| Team with shared CI | `api-key` |
| Rotating credentials | `api-key` (keys can be revoked individually) |

You can use different modes locally vs CI by maintaining separate `.env` files or overriding env vars in CI. The `sign.toml` only names the env vars — the values come from the environment.
