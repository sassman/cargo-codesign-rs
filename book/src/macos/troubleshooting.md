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

In CI, this almost always means the keychain wasn't unlocked, or the partition list didn't grant `codesign` access. `--ci-import-cert` handles both for you (`security unlock-keychain` immediately after `create-keychain`, plus `set-key-partition-list -S apple-tool:,apple:,codesign:`), so this should not occur when using the standard CI flow.

**If you hit it anyway:**
- Confirm `--ci-import-cert` ran successfully and `target/.codesign-keychain` exists with the absolute path of the ephemeral keychain.
- Re-run with `--verbose` to see the exact `security` and `codesign` commands.
- See the [GitHub Actions walkthrough](../ci/github-actions.md) for the canonical setup.

## Notarization times out

Apple's notarization service occasionally takes longer than usual.

**Fix:** Re-run the command. `xcrun notarytool submit --wait` has a built-in timeout. If it consistently times out, check [Apple's system status](https://developer.apple.com/system-status/).

## "No signing identity found"

`cargo codesign status` shows `tool:codesign` as available but signing fails.

**Local dev:**
- Verify the certificate is installed: `security find-identity -v -p codesigning`
- Check the identity string in `sign.toml` matches. The default `"Developer ID Application"` matches any Developer ID Application certificate.

**CI:** `--ci-import-cert` creates an ephemeral keychain at an absolute path *and* prepends it to the user keychain search list, so `security find-identity -v -p codesigning` should list your Developer ID Application identity right after import. If it doesn't, inspect the ephemeral keychain directly:

```bash
security find-identity -v -p codesigning "$(cat target/.codesign-keychain)"
```

If that's also empty, the `.p12` is probably missing its private key — re-export the **identity** from Keychain Access (with the disclosure triangle expanded so the private key is included) instead of just the certificate.

## Verbose output

When debugging, always use `--verbose` to see the exact subprocess commands:

```bash
cargo codesign macos --app "MyApp.app" --verbose
```

This prints every `codesign`, `hdiutil`, `xcrun`, and `stapler` invocation with its arguments and output.
