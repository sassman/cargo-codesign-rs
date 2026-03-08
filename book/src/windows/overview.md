# Windows Signing

> Windows signing via Azure Trusted Signing is planned but not yet implemented.

When available, `cargo codesign windows` will:

- Discover binaries via `cargo metadata`
- Sign `.exe` files using Azure Trusted Signing (`signtool.exe` + Azure Code Signing DLib)
- Verify signatures after signing
- Output signed binaries to `target/signed/`

Configuration will use the `[windows]` and `[windows.env]` sections of `sign.toml`:

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

See the [sign.toml Reference](../reference/sign-toml.md) for details.
