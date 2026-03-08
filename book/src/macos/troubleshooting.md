# Troubleshooting macOS Signing

## "Apple could not verify" dialog

Users see this when the app is not notarized or the ticket is not stapled.

**Fix:** Make sure you're running the full pipeline without `--skip-notarize`:

```bash
cargo codesign macos --app "MyApp.app"
```

If notarization succeeded but users still see the dialog, the DMG may not have been stapled. Re-run without `--skip-staple`.

## "MyApp.app is damaged and can't be opened"

This usually means the code signature is invalid — often because the `.app` was modified after signing (e.g., by `xattr` quarantine stripping, or by copying incorrectly).

**Fix:** Re-sign after any modification to the bundle.

## Notarization fails with "The signature of the binary is invalid"

The binary inside the `.app` wasn't signed with the hardened runtime.

**Fix:** cargo-codesign always uses `--options runtime` so this shouldn't happen. If it does, check that:
- You're not signing with a different tool before cargo-codesign
- The `.app` bundle structure is correct (binary in `Contents/MacOS/`)

## Notarization fails with "The executable does not have the hardened runtime enabled"

Same as above — the hardened runtime flag is missing.

## "errSecInternalComponent" or keychain errors

This typically happens in CI when the keychain isn't properly configured.

**Fix for CI:**
1. Make sure the keychain is unlocked before signing
2. The `security set-key-partition-list` call must include `codesign:` in the `-S` flag
3. See the [GitHub Actions walkthrough](../ci/github-actions.md) for the correct keychain setup

## Notarization times out

Apple's notarization service occasionally takes longer than usual.

**Fix:** Re-run the command. `xcrun notarytool submit --wait` has a built-in timeout. If it consistently times out, check [Apple's system status](https://developer.apple.com/system-status/).

## "No signing identity found"

`cargo codesign status` shows `tool:codesign` as available but signing fails.

**Fix:**
- Verify the certificate is installed: `security find-identity -v -p codesigning`
- Check the identity string in `sign.toml` matches. The default `"Developer ID Application"` matches any Developer ID Application certificate.
- In CI, make sure the certificate was imported into the correct keychain

## Verbose output

When debugging, always use `--verbose` to see the exact subprocess commands:

```bash
cargo codesign macos --app "MyApp.app" --verbose
```

This prints every `codesign`, `hdiutil`, `xcrun`, and `stapler` invocation with its arguments and output.
