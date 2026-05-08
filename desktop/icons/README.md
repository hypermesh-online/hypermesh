# Tauri icons

Tauri 2 expects the following files in this directory; they are
referenced by `../tauri.conf.json` under `bundle.icon` and `app.trayIcon`:

| File | Use |
|------|-----|
| `32x32.png` | Window icon (small) |
| `128x128.png` | Window icon (standard) |
| `128x128@2x.png` | Window icon (HiDPI) |
| `icon.png` | Tray icon (Linux/Windows fallback) |
| `icon.icns` | macOS app bundle |
| `icon.ico` | Windows installer + .exe |
| `tray-green.png` | Tray icon: daemon running |
| `tray-yellow.png` | Tray icon: daemon starting |
| `tray-red.png` | Tray icon: daemon stopped/error |

## Generating

For C.3 alpha these files are placeholders — the desktop shell falls
back to the default Tauri icon if any are missing. Generate real
icons before the first public C.3 release:

```bash
# Tauri ships an icon generator that produces every required size +
# format from a single 1024x1024 PNG.
cargo tauri icon path/to/source-1024.png
```

Tray state-icons (green/yellow/red) need to be authored separately —
they should be 22x22 (Linux/Windows) or 16x16/32x32@2x (macOS template).
The tray code in `../src/tray.rs` resolves them via Tauri's resource
path helper, so silently no-ops if the asset isn't bundled.
