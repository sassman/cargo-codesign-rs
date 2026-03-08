# Windows Signing

Sign Windows executables using [Azure Trusted Signing](https://learn.microsoft.com/en-us/azure/trusted-signing/) via `signtool.exe`.

## Usage

```bash
cargo codesign windows
```

This will:

1. Load `sign.toml` and read the `[windows]` section
2. Discover binaries via `cargo metadata`
3. Generate `metadata.json` for Azure Trusted Signing
4. Sign each `.exe` with `signtool.exe` using SHA-256 + timestamp
5. Clean up temporary metadata files

### Install tools automatically

On a fresh CI runner, use `--install-tools` to download the Azure Code Signing DLib via NuGet:

```bash
cargo codesign windows --install-tools
```

## Configuration

```toml
[windows]
timestamp-server = "http://timestamp.acs.microsoft.com"

[windows.env]
tenant-id      = "AZURE_TENANT_ID"
client-id      = "AZURE_CLIENT_ID"
client-secret  = "AZURE_CLIENT_SECRET"
endpoint       = "AZURE_SIGNING_ENDPOINT"
account-name   = "AZURE_SIGNING_ACCOUNT_NAME"
cert-profile   = "AZURE_SIGNING_CERT_PROFILE"
```

See the [sign.toml Reference](../reference/sign-toml.md) for full details, and [Setting Up Credentials](./credentials.md) for how to obtain Azure Trusted Signing credentials.
