# Setting Up Windows Credentials

To sign Windows executables with Azure Trusted Signing, you need six credentials. All are set as environment variables (or in `.env`).

## 1. Azure AD tenant

Register an Azure AD application for Trusted Signing.

| Credential | Env var | Where to find it |
|---|---|---|
| Tenant ID | `AZURE_TENANT_ID` | Azure Portal > Azure Active Directory > Overview > Tenant ID |
| Client ID | `AZURE_CLIENT_ID` | Azure Portal > App registrations > your app > Application (client) ID |
| Client Secret | `AZURE_CLIENT_SECRET` | Azure Portal > App registrations > your app > Certificates & secrets > New client secret |

## 2. Trusted Signing account

Create a Trusted Signing account and certificate profile in the Azure Portal.

| Credential | Env var | Where to find it |
|---|---|---|
| Endpoint | `AZURE_SIGNING_ENDPOINT` | Azure Portal > Trusted Signing > your account > Overview > Endpoint |
| Account name | `AZURE_SIGNING_ACCOUNT_NAME` | The name you chose when creating the Trusted Signing account |
| Certificate profile | `AZURE_SIGNING_CERT_PROFILE` | Azure Portal > Trusted Signing > Certificate profiles > profile name |

## 3. Tools

On the CI runner (Windows):

- **signtool.exe** — part of the Windows SDK. Usually available on `windows-latest` GitHub runners.
- **Azure.CodeSigning.Dlib.dll** — install with `cargo codesign windows --install-tools` or manually via `nuget install Microsoft.Trusted.Signing.Client`.

## Verify

After setting all six variables, run:

```bash
cargo codesign status
```
