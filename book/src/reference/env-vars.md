# Environment Variables

cargo-codesign reads secret values exclusively from environment variables. The `sign.toml` file stores env var **names**, never values.

## .env auto-loading

cargo-codesign uses [dotenvy](https://crates.io/crates/dotenvy) to auto-load a `.env` file from the current directory. This makes local development convenient:

```bash
# .env (never commit)
APPLE_ID=you@example.com
APPLE_TEAM_ID=ABCDE12345
APPLE_APP_PASSWORD=xxxx-xxxx-xxxx-xxxx
```

## macOS — apple-id mode

| Env var (default name) | Description |
|------------------------|-------------|
| `APPLE_ID` | Apple ID email address |
| `APPLE_TEAM_ID` | 10-character Apple Developer Team ID |
| `APPLE_APP_PASSWORD` | App-specific password for notarization |

## macOS — api-key mode

| Env var (default name) | Description |
|------------------------|-------------|
| `MACOS_CERTIFICATE` | Base64-encoded `.p12` certificate |
| `MACOS_CERTIFICATE_PASSWORD` | Password for the `.p12` file |
| `APPLE_NOTARIZATION_KEY` | Base64-encoded `.p8` App Store Connect API key |
| `APPLE_NOTARIZATION_KEY_ID` | API key ID (from App Store Connect) |
| `APPLE_NOTARIZATION_ISSUER_ID` | Issuer ID (from App Store Connect) |

## Windows

| Env var (default name) | Description |
|------------------------|-------------|
| `AZURE_TENANT_ID` | Azure AD tenant ID |
| `AZURE_CLIENT_ID` | Azure AD client/application ID |
| `AZURE_CLIENT_SECRET` | Azure AD client secret |
| `AZURE_SIGNING_ENDPOINT` | Azure Trusted Signing endpoint URL |
| `AZURE_SIGNING_ACCOUNT_NAME` | Trusted Signing account name |
| `AZURE_SIGNING_CERT_PROFILE` | Certificate profile name |

## Linux

| Env var (default name) | Description |
|------------------------|-------------|
| `COSIGN_PRIVATE_KEY` | Cosign private key (for cosign method) |

## Update signing

| Env var (default name) | Description |
|------------------------|-------------|
| `UPDATE_SIGNING_KEY` | Base64-encoded Ed25519 private key |

## Custom env var names

All env var names are configurable in `sign.toml`. The names above are conventions — you can use any name:

```toml
[macos.env]
apple-id = "MY_CUSTOM_APPLE_ID_VAR"
```
