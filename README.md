# Verlox

Roblox version launcher and downloader for Windows, built with Tauri 2 and React.

## Features

- View live Roblox version hashes (latest / previous / upcoming) from WEAO and the official Roblox API, with a 5-minute cache.
- Copy hashes and download any version by custom hash, with caching, MD5 verification, and `.part` resume support.
- Browse executor compatibility information and download the Roblox version an executor supports.
- Scan, launch, open, and remove locally installed versions under `%LOCALAPPDATA%\Roblox\Versions`.
- Vietnamese-language logs and a desktop UI built with React 19, Vite, and Lucide icons.

The earlier Python/PySide6 version is kept in `backup/` for reference only.

## Requirements

- Node.js and npm
- Rust stable (MSVC toolchain) and Cargo
- Visual Studio Build Tools with the C++ workload (for the MSVC linker)
- WebView2 Runtime (preinstalled on modern Windows 10/11)

## Development

```powershell
npm install
npm run dev        # Tauri dev app (starts the Vite server automatically)
```

## Build

```powershell
npm run build:portable   # standalone click-and-run exe (no installer)
npm run build            # optional NSIS installer
```

The portable build outputs `src-tauri/target/release/verlox.exe`. It embeds the
frontend and needs no installation — just double-click it. The target PC still
needs the WebView2 Runtime (preinstalled on modern Windows 10/11).

Useful commands:

```powershell
npm run ui:build   # frontend only (outputs to dist/)
npm run ui:dev     # Vite dev server only
cargo test         # from src-tauri/ — backend unit tests
cargo check        # from src-tauri/ — fast compile check
```

## CI

`.github/workflows/build.yml` builds the portable Windows exe on every push to `main`, on pull requests, and on manual runs (Actions → **Build Windows portable exe** → Run workflow). The finished file is attached to the workflow run as `Verlox-Windows-x64-portable` (`Verlox-<version>-portable.exe`). Rust dependencies are cached between runs, so later builds are much faster.

Pushing a version tag such as `v1.2.0` also creates a draft GitHub release containing the portable exe.

## License

MIT — see [License](License).
