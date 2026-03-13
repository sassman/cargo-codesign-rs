# DMG Installer Styling

Create polished drag-to-install DMG images with a background image and positioned icons.

## Before and after

Without `[macos.dmg]` config, the DMG opens with Finder's default layout — no background, no icon positioning, no visual guidance for the user.

With styling configured, the DMG opens with your background image, the app icon and Applications folder at exact positions, creating the standard macOS "drag to install" experience.

## Setup

### 1. Create a background image

Design a background image for your installer window. Common dimensions are 660x400 pixels. PNG format is recommended.

The image should visually guide users to drag the app icon to the Applications folder. A subtle arrow or instructional text works well.

### 2. Add the `[macos.dmg]` section to `sign.toml`

```toml
[macos.dmg]
background = "assets/dmg-background.png"
window-size = [660, 400]
icon-size = 128
app-position = [160, 200]
app-drop-link = [500, 200]
```

### 3. Build as usual

```bash
cargo codesign macos --app "target/release/bundle/MyApp.app"
```

The DMG is created with the styled layout automatically. No additional flags or steps needed.

## Configuration reference

| Field | Type | Example | Description |
|-------|------|---------|-------------|
| `background` | path | `"assets/dmg-background.png"` | Background image path, relative to where `cargo codesign` runs. |
| `window-size` | `[u32, u32]` | `[660, 400]` | Width and height of the Finder window in pixels. |
| `icon-size` | integer | `128` | Size of icons in the Finder window. |
| `app-position` | `[u32, u32]` | `[160, 200]` | Pixel coordinates of the `.app` icon. |
| `app-drop-link` | `[u32, u32]` | `[500, 200]` | Pixel coordinates of the `Applications` symlink. |

All fields are required when `[macos.dmg]` is present. Omit the entire section for a plain DMG.

### Coordinate system

Coordinates are relative to the top-left corner of the Finder window content area. The y-axis points downward.

For a 660x400 window with two icons side by side:
- Left icon (your app): `[160, 200]` — centered vertically, left third
- Right icon (Applications): `[500, 200]` — centered vertically, right third

Adjust these to match your background image design.

### Background image tips

- Use the same dimensions as `window-size` for a pixel-perfect fit
- PNG format is recommended (lossless, supported everywhere)
- The image is copied into the DMG as `.background/bg.png` — this hidden folder is standard macOS convention
- Keep the file size reasonable; the image is embedded in the final DMG

## How it works

cargo-codesign writes a native `.DS_Store` file directly into the DMG staging directory. This file tells Finder how to display the window — icon positions, icon size, background image reference, and window dimensions.

The `.DS_Store` is written in macOS's buddy-allocator B-tree format and includes:
- **Iloc** records — icon positions for each file
- **bwsp** record — window size and position (binary plist)
- **icvp** record — icon view settings with embedded Alias V2 pointing to the background image (binary plist)
- **pBBk** record — Bookmark data for the background image path
- **vSrn** record — Finder view version

A single `hdiutil create -format UDZO` call produces the final compressed DMG.

### Why not AppleScript?

The traditional approach mounts a read-write DMG, launches Finder via AppleScript to configure the window, then converts to read-only. This is:

- **Slow** — Finder launch adds ~15 seconds
- **Flaky** — AppleScript timeouts are common in CI and headless environments
- **Non-deterministic** — Finder writes variable metadata

The native `.DS_Store` writer avoids all of this. It runs in milliseconds, works headless, and produces deterministic output.

## Troubleshooting

### Background image doesn't appear

- Verify the `background` path exists relative to your working directory
- Check that the image is a valid PNG (try opening it in Preview)
- Ensure `window-size` matches or is close to your image dimensions

### Icons are in the wrong position

- Coordinates are in pixels from the top-left of the window content area
- Try adjusting positions in increments of 20-40px
- Open the DMG and use Finder's "Show View Options" (Cmd+J) to see the current grid settings

### Plain DMG created instead of styled

- Confirm `[macos.dmg]` is present in your `sign.toml`
- All five fields are required — a missing field will cause a parse error
- Run with `--verbose` to see the `.DS_Store` generation message
