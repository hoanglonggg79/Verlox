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
npm run build      # builds the frontend and produces the NSIS installer
```

Useful commands:

```powershell
npm run ui:build   # frontend only (outputs to dist/)
npm run ui:dev     # Vite dev server only
cargo test         # from src-tauri/ — backend unit tests
cargo check        # from src-tauri/ — fast compile check
```

## License

MIT — see [License](License).
