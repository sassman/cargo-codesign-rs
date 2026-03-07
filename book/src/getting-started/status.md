# Checking Your Setup

Before signing anything, verify that your credentials and tools are in order:

```bash
cargo codesign status
```

Example output when everything is configured:

```
Using config: /path/to/project/sign.toml

  ✓ env:APPLE_ID: set
  ✓ env:APPLE_TEAM_ID: set
  ✓ env:APPLE_APP_PASSWORD: set
  ✓ tool:codesign: /usr/bin/codesign
  ✓ tool:xcrun: /usr/bin/xcrun
  ✓ tool:hdiutil: /usr/bin/hdiutil

All checks passed.
```

Example output with missing credentials:

```
Using config: /path/to/project/sign.toml

  ✗ env:APPLE_ID: APPLE_ID: not set
  ✓ env:APPLE_TEAM_ID: set
  ✗ env:APPLE_APP_PASSWORD: APPLE_APP_PASSWORD: not set
  ✓ tool:codesign: /usr/bin/codesign
  ✓ tool:xcrun: /usr/bin/xcrun
  ✓ tool:hdiutil: /usr/bin/hdiutil

2 check(s) failed.
```

## What it checks

- **Environment variables**: For each env var name listed in your `sign.toml`, checks that it's set and non-empty.
- **Platform tools**: Checks that `codesign`, `xcrun`, and `hdiutil` are on your PATH (macOS). Windows checks for `signtool.exe`.
- Only checks credentials relevant to your configured auth mode — `api-key` mode checks different vars than `apple-id` mode.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 2 | Configuration error (missing `sign.toml`, bad format) |
| 4 | One or more checks failed |

## Loading credentials from `.env`

cargo-codesign automatically loads `.env` from the current directory (via [dotenvy](https://crates.io/crates/dotenvy)). This means you can keep your credentials in `.env` for local development:

```bash
# .env — never commit this file
APPLE_ID=you@example.com
APPLE_TEAM_ID=ABCDE12345
APPLE_APP_PASSWORD=xxxx-xxxx-xxxx-xxxx
```

Make sure `.env` is in your `.gitignore`.
