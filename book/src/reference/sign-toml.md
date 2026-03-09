# sign.toml Reference

`sign.toml` is the configuration file for cargo-codesign. It maps platform signing settings to environment variable names.

## File location

cargo-codesign looks for config in this order:

1. `--config <PATH>` flag (explicit)
2. `./sign.toml` (project root)
3. `./.cargo/sign.toml` (fallback)

If both `./sign.toml` and `./.cargo/sign.toml` exist, `./sign.toml` wins and a warning is emitted.

## Full example

```toml
# sign.toml — cargo-codesign configuration
# Generate with: cargo codesign init (coming soon)

[macos]
identity = "Developer ID Application"
entitlements = "entitlements.plist"
auth = "api-key"    # "api-key" (CI) or "apple-id" (local/indie)

[macos.env]
# api-key mode
certificate          = "APPLE_CERTIFICATE"
certificate-password = "APPLE_CERTIFICATE_PASSWORD"
notarization-key     = "APPLE_NOTARIZATION_KEY"
notarization-key-id  = "APPLE_NOTARIZATION_KEY_ID"
notarization-issuer  = "APPLE_NOTARIZATION_ISSUER_ID"
# apple-id mode
apple-id     = "APPLE_ID"
team-id      = "APPLE_TEAM_ID"
app-password = "APPLE_APP_PASSWORD"

[windows]
timestamp-server = "http://timestamp.acs.microsoft.com"

[windows.env]
tenant-id      = "AZURE_TENANT_ID"
client-id      = "AZURE_CLIENT_ID"
client-secret  = "AZURE_CLIENT_SECRET"
endpoint       = "AZURE_SIGNING_ENDPOINT"
account-name   = "AZURE_SIGNING_ACCOUNT_NAME"
cert-profile   = "AZURE_SIGNING_CERT_PROFILE"

[linux]
method = "cosign"     # cosign | minisign | gpg

[linux.env]
key = "COSIGN_PRIVATE_KEY"

[update]
public-key = "update-signing.pub"

[update.env]
signing-key = "UPDATE_SIGNING_KEY"

[status]
cert-warn-days = 60
cert-error-days = 7
```

## Sections

### `[macos]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `identity` | string | `"Developer ID Application"` | Signing identity substring |
| `entitlements` | path | none | Path to entitlements plist |
| `auth` | `"api-key"` or `"apple-id"` | required | Notarization auth mode |

### `[macos.env]`

Maps credential fields to environment variable names. Which fields are required depends on the `auth` mode:

**`apple-id` mode:**

| Field | Required | Description |
|-------|----------|-------------|
| `apple-id` | yes | Env var for Apple ID email |
| `team-id` | yes | Env var for team ID |
| `app-password` | yes | Env var for app-specific password |

**`api-key` mode:**

| Field | Required | Description |
|-------|----------|-------------|
| `certificate` | yes | Env var for base64 .p12 certificate |
| `certificate-password` | yes | Env var for .p12 password |
| `notarization-key` | yes | Env var for base64 .p8 API key |
| `notarization-key-id` | yes | Env var for API key ID |
| `notarization-issuer` | yes | Env var for issuer ID |

### `[windows]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `timestamp-server` | string | none | Timestamp server URL |

### `[windows.env]`

| Field | Description |
|-------|-------------|
| `tenant-id` | Azure tenant ID |
| `client-id` | Azure client ID |
| `client-secret` | Azure client secret |
| `endpoint` | Azure signing endpoint |
| `account-name` | Azure signing account name |
| `cert-profile` | Azure certificate profile |

### `[linux]`

| Field | Type | Description |
|-------|------|-------------|
| `method` | `"cosign"`, `"minisign"`, or `"gpg"` | Signing method |

### `[linux.env]`

| Field | Description |
|-------|-------------|
| `key` | Signing key env var |

### `[update]`

| Field | Type | Description |
|-------|------|-------------|
| `public-key` | path | Path to public key file |

### `[update.env]`

| Field | Description |
|-------|-------------|
| `signing-key` | Env var for base64 Ed25519 private key |

### `[status]`

| Field | Type | Description |
|-------|------|-------------|
| `cert-warn-days` | integer | Warn when cert expires within N days |
| `cert-error-days` | integer | Error when cert expires within N days |

## Strict parsing

All sections use `deny_unknown_fields` — typos in field names cause a clear parse error rather than being silently ignored.
