# Setting Up macOS Credentials

## Developer ID Application Certificate

You need a **Developer ID Application** certificate (not a Mac App Store certificate — that's for App Store distribution, which cargo-codesign does not handle).

### Getting the certificate

1. **Join the Apple Developer Program** (99 EUR/year) at [developer.apple.com/programs](https://developer.apple.com/programs)

2. **Create a Certificate Signing Request (CSR):**
   - Open **Keychain Access** → Certificate Assistant → **Request a Certificate From a Certificate Authority**
   - Enter your Apple ID email, leave CA Email empty, select **Saved to disk**
   - Save the `.certSigningRequest` file

3. **Upload to Apple Developer Portal:**
   - Go to [developer.apple.com/account/resources/certificates](https://developer.apple.com/account/resources/certificates)
   - Click **+** → Select **Developer ID Application**
   - Upload your `.certSigningRequest` file
   - Download the `.cer` file

4. **Install the certificate:**
   - Double-click the `.cer` file to import into Keychain Access
   - Verify: `security find-identity -v -p codesigning`

### For CI: exporting a .p12

CI runners don't have your Keychain. Export the certificate as a `.p12` file:

```bash
# In Keychain Access: right-click the certificate → Export Items → .p12
# Or use openssl:
openssl pkcs12 -export \
  -out certificate.p12 \
  -inkey private-key.key \
  -in certificate.pem \
  -password pass:YOUR_PASSWORD
```

Base64-encode it for a GitHub Secret:

```bash
base64 -i certificate.p12 | pbcopy
```

Store as `APPLE_CERTIFICATE_BASE64` in your repo's GitHub Secrets.

## Notarization Credentials

cargo-codesign supports two auth modes for notarization. Choose based on your setup:

| Mode | Best for | Credentials needed |
|------|----------|-------------------|
| `apple-id` | Local development, indie devs | Apple ID + app-specific password |
| `api-key` | CI, teams, automation | App Store Connect API key (.p8) |

### apple-id mode

1. Go to [account.apple.com](https://account.apple.com) → Sign-In and Security → App-Specific Passwords
2. Generate a password labeled "cargo-codesign" (or similar)
3. Store it:

```bash
# .env (local) or GitHub Secrets (CI)
APPLE_ID=you@example.com
APPLE_TEAM_ID=ABCDE12345
APPLE_APP_PASSWORD=xxxx-xxxx-xxxx-xxxx
```

### api-key mode

1. Go to [appstoreconnect.apple.com/access/integrations/api](https://appstoreconnect.apple.com/access/integrations/api)
2. Create a new key with **Developer** access
3. Download the `.p8` file (you can only download it once)
4. Note the **Key ID** and **Issuer ID**
5. Store them:

```bash
# Base64-encode the .p8 for CI secrets
APPLE_NOTARIZATION_KEY=$(base64 -i AuthKey_XXXXXXXXXX.p8)
APPLE_NOTARIZATION_KEY_ID=XXXXXXXXXX
APPLE_NOTARIZATION_ISSUER_ID=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
```

See [Auth Modes](./auth-modes.md) for how these map to `sign.toml`.
