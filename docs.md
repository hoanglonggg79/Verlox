# VERLOX — Roblox Version Launcher

> Tài liệu dự án, viết cho người mới tiếp nhận (onboarding). Trạng thái tài liệu: **sau bước 2** (scaffold + port logic thuần + engine download + UI progress), cập nhật 2026-11-05.

---

## 1. Verlox là gì

Launcher & version manager cho Roblox Player/Studio trên Windows:

- Xem hash **version trực tiếp** (mới nhất / trước đó / sắp tới) từ WEAO + API chính thức của Roblox.
- **Sao chép hash** mới nhất, tải version bất kỳ bằng **custom hash** hoặc chọn từ **DeployHistory** (30 bản gần đây).
- Quét **các version đã cài trên máy** (`%LOCALAPPDATA%\Roblox\Versions`), chạy / mở thư mục / xóa từng version.
- Cache download + MD5 verify + resume (`.part`), giống bản Python cũ.
- Lịch sử download tương thích ngược với file `download_history.json` của bản Python.

Tiền thân: **Roblox Version Downloader 1.2** (Python + PySide6) — toàn bộ nằm trong `backup/`, chỉ để tham khảo.

## 2. Stack & môi trường

| Thành phần | Vai trò |
|---|---|
| Rust stable (MSVC) + Cargo | Backend — mọi logic download/thư mục/registry |
| Tauri 2 | Khung desktop (webview WebView2, IPC commands + events) |
| HTML/CSS/JS thuần (không framework, không bundler) | Frontend trong `src/`, dùng `window.__TAURI__` (withGlobalTauri) |
| Node.js + npm | Cần cho `@tauri-apps/cli` (chạy `tauri dev` / `tauri build`) |

Môi trường đã verify trên máy này (2026-11-05): Node v24.19.0, npm 11.17.0, rustc 1.98.0, cargo 1.98.0. `cargo check` pass 0 warning, `cargo test` 19/19 pass (sau bước 2), 455 crates đã lock trong `src-tauri/Cargo.lock`.

Yêu cầu thêm cho máy khác: **VS Build Tools (MSVC linker)** khi build release, **WebView2 Runtime** (Windows 10/11 có sẵn).

### Lệnh

```powershell
npm install                 # 1 lần, cài @tauri-apps/cli
npm run dev                 # chạy app ở chế độ dev (Tauri serve src/ trực tiếp)
npm run build               # build NSIS installer (bước 5 của lộ trình)
cargo test                  # trong src-tauri — test logic thuần (hash, manifest, unsafe-dir, history)
cargo check                 # trong src-tauri — verify compile nhanh
```

## 3. Cấu trúc thư mục

```
RobloxVersionDownloader/   (đề xuất: đổi tên repo thành verlox/)
├── docs.md                # file này — đọc trước khi code
├── package.json           # npm scripts + tauri CLI
├── API_Docs.txt           # tham chiếu API WEAO (bản update 2026-11, có cả endpoint /future)
├── License                # MIT
├── icon.ico               # icon gốc (bản sao nằm trong src-tauri/icons/)
├── src/                   # FRONTEND — HTML/CSS/JS tĩnh
│   ├── index.html         # layout các section
│   ├── styles.css         # dark theme; PALETTE cũ của PySide6 thành CSS variables ở :root
│   └── main.js            # invoke commands + render list; không dùng framework
├── src-tauri/
│   ├── Cargo.toml         # deps: tauri 2, reqwest (rustls), zip, md-5, chrono, windows-registry...
│   ├── tauri.conf.json    # productName Verlox, frontendDist ../src, withGlobalTauri true, csp null (bước 5 harden)
│   ├── capabilities/default.json   # ACL Tauri 2 (core:default)
│   ├── icons/icon.ico
│   └── src/
│       ├── main.rs        # entry, gọi verlox_lib::run()
│       ├── lib.rs        # toàn bộ #[tauri::command] + menu commands
│       ├── hash.rs       # validate_hash — đã port + test
│       ├── manifest.rs   # parse_manifest + PACKAGE_MAP + APP_SETTINGS_XML — đã port + test
│       ├── versions.rs   # WEAO (current/future/past) + API chính thức + DeployHistory — đã port
│       ├── installed.rs  # scan Versions/, guard thư mục unsafe — đã port + test
│       ├── history.rs    # download_history.json đọc/ghi + push — đã port + test
│       ├── cdn.rs       # probe CDN + download resume + MD5 + progress — đã port + test (bước 2)
│       ├── extract.rs   # giải nén zip theo PACKAGE_MAP + WebView2 installer + AppSettings.xml — đã port + test (bước 2)
│       └── (bước 3) win.rs      # đăng ký protocol roblox-player:// qua windows-registry
└── backup/                # DỰ ÁN PYTHON CŨ — chỉ tham khảo, KHÔNG build/edit
    ├── RobloxVersionDownloader1.2.py   # nguồn port chính (~1660 dòng, có sẵn mọi logic cần)
    ├── README.md                      # mô tả bản 1.2
    ├── download_history.json          # dữ liệu lịch sử cũ (import bằng lệnh trong app)
    └── CleanerCache/CleanRobloxCache.bat
```

## 4. Kiến trúc runtime

Frontend gọi backend qua `invoke("<command>", {args-camelCase})`; backend trả JSON. Structs Rust serialize `snake_case` field + `#[serde(rename_all = "camelCase")]` → JS đọc camelCase (`studioLatest`, `sizeBytes`...). Command args JS dùng camelCase (`withSizes`, `installDir`, `filePath`) — Tauri 2 tự map sang snake_case của Rust.

Bảng commands hiện có (`src-tauri/src/lib.rs`):

| Command | Trạng thái | Ý nghĩa |
|---|---|---|
| `get_live_versions` | ✅ chạy được | WEAO current/past/future + API chính thức → `{latest, studioLatest, futureWindows, previousWindows, previousMac}` |
| `get_history_versions` | ✅ chạy được | Quét `DeployHistory.txt` → tối đa 30 hash |
| `list_installed_versions(withSizes)` | ✅ chạy được | Scan `%LOCALAPPDATA%\Roblox\Versions` → hash, path, có Player/Studio exe, dung lượng |
| `launch_version(hash)` | ✅ chạy được | Spawn `RobloxPlayerBeta.exe` (working dir = thư mục version) |
| `open_folder(path)` | ✅ chạy được | Mở Explorer |
| `delete_version(hash)` | ✅ chạy được | Xóa thư mục version (chỉ chấp nhận dir tên đúng format hash) |
| `clear_cache()` | ✅ chạy được | Dọn `%LOCALAPPDATA%\Roblox\Downloads`, trả về số byte giải phóng; trả lỗi nếu đang có tác vụ tải (chống xóa `.part` đang ghi) |
| `get_download_history` | ✅ chạy được | Đọc `%LOCALAPPDATA%\Verlox\download_history.json` |
| `import_legacy_history(filePath)` | ✅ chạy được | Merge JSON cũ (backup/download_history.json) vào history mới |
| `create_shortcut(hash)` | ✅ chạy được | Tạo `.lnk` trên Desktop qua PowerShell COM |
| `start_download(hash, installDir, options)` | ✅ chạy được | Spawn task download: probe CDN → manifest → tải từng zip (cache MD5, resume `.part`, verify MD5) → giải nén theo PACKAGE_MAP → ghi AppSettings.xml → shortcut tùy chọn → `history::push`. `installDir` là thư mục gốc, version cài vào con `{hash}`. Trả lỗi `EXISTS::<dir>` khi thư mục đích đã có nội dung và `options.overwrite` chưa bật (UI hỏi người dùng rồi gọi lại với `overwrite: true`) |
| `cancel_download` | ✅ chạy được | Set cờ cancel — task dừng ở điểm kiểm tra kế tiếp, các file `.part` được giữ để resume lần sau |
| `register_protocol` | ⛔ stub — "bước 3" | Ghi registry `HKCU\Software\Classes\roblox{,-player}` |

Event dùng thật ở bước 2 (frontend `window.__TAURI__.event.listen`): `download://log` `{message, level}` (level: `info/muted/warning/success/error`), `download://progress` `{overallPct, pkgName, pkgPct, speedBps, etaSecs}`, `download://done` `{installDir}` (`null` nếu bị hủy giữa chừng), `download://error` `{title, message}`.

## 5. Domain knowledge Roblox (port từ Python 1.2)

### 5.1 Version hash

- Format: `version-` + **16 ký tự hex** (vd `version-eb4648e9148d440c`). Người dùng nhập thiếu prefix app tự thêm.
- `1.2.3.4567` là số version, **không phải hash** — từ chối.
- Rust: `hash::validate_hash` + test `src-tauri/src/hash.rs`.

### 5.2 CDN (5 endpoint, probe chọn nhanh nhất — làm ở bước 2)

| Endpoint | Ghi chú |
|---|---|
| `https://setup.rbxcdn.com` | primary |
| `https://setup-ak.rbxcdn.com` | fallback |
| `https://setup-cf.rbxcdn.com` | fallback |
| `https://roblox-setup.cachefly.net` | fallback |
| `https://s3.amazonaws.com/setup.roblox.com` | fallback |

- Manifest: `{cdn}/{hash}-rbxPkgManifest.txt`.
- Package: `{cdn}/{hash}-{tên-zip}`.
- Lịch sử deploy: `{primary}/DeployHistory.txt`.

### 5.3 Format manifest

Dòng đầu có thể là marker `v0`/`v1`/`v2` (hoặc một dòng không chứa dấu `.`) — bỏ qua. Sau đó mỗi **4 dòng** là 1 package: `tên file zip`, `md5`, `kích thước nén`, `kích thước giải nén` (không phải số → coi là 0). Rust: `manifest::parse_manifest` + test.

### 5.4 PACKAGE_MAP — zip giải nén vào đâu (trong thư mục install)

| Zip | Đích |
|---|---|
| RobloxApp.zip, redist.zip, WebView2.zip | `.` (gốc) |
| shaders.zip → `shaders`; ssl.zip → `ssl` | |
| WebView2RuntimeInstaller.zip | `__webview2_installer__` (chạy exe /silent /install nếu bật option) |
| content-avatar.zip | `content/avatar` |
| content-configs.zip | `content/configs` |
| content-fonts.zip | `content/fonts` |
| content-sky.zip | `content/sky` |
| content-sounds.zip | `content/sounds` |
| content-textures2.zip | `content/textures` |
| content-models.zip | `content/models` |
| content-platform-fonts.zip | `PlatformContent/pc/fonts` |
| content-platform-dictionaries.zip | `PlatformContent/pc/shared_compression_dictionaries` |
| content-terrain.zip | `PlatformContent/pc/terrain` |
| content-textures3.zip | `PlatformContent/pc/textures` |
| extracontent-luapackages.zip | `ExtraContent/LuaPackages` |
| extracontent-translations.zip | `ExtraContent/translations` |
| extracontent-models.zip | `ExtraContent/models` |
| extracontent-textures.zip | `ExtraContent/textures` |
| extracontent-places.zip | `ExtraContent/places` |

Nguồn chuẩn: `src-tauri/src/manifest.rs` (`PACKAGE_MAP`), khớp 1:1 với `PACKAGE_MAP` trong `backup/RobloxVersionDownloader1.2.py`.

### 5.5 AppSettings.xml

Sau khi giải nén xong, ghi đè `{install_dir}/AppSettings.xml` với nội dung const `manifest::APP_SETTINGS_XML` (ContentFolder=content, BaseUrl=www.roblox.com).

### 5.6 Thư mục

| Vai trò | Đường dẫn |
|---|---|
| Cài đặt bản chính thức | `%LOCALAPPDATA%\Roblox\Versions\<hash>\` (chứa `RobloxPlayerBeta.exe` / `RobloxStudioBeta.exe`) |
| Cache zip đã tải | `%LOCALAPPDATA%\Roblox\Downloads\` (mỗi file `<hash>-<tên-zip>`, đang tải thì `*.part`) |
| Lịch sử Verlox | `%LOCALAPPDATA%\Verlox\download_history.json` |

Format history (tương thích bản Python): JSON array `[{ "hash", "path", "time" }]`, tối đa 20 bản ghi.

### 5.7 An toàn thư mục (chống ghi đè lên hệ thống)

Rust: `installed::is_unsafe_dir` + test.

- **Exact-match chặn**: `C:\Windows`, `C:\Windows\System32`, `C:\Program Files`, `C:\Program Files (x86)`, `C:\`, `/`, `/usr`, `/etc`, `/bin`, `/system`, `/library`.
- **Prefix chặn** (con của): Windows, System32, /usr, /etc, /bin, /system, /library.
- Cho phép cài vào_places như `C:\Program Files\Roblox` (tạm được phép như bản Python).

## 6. Nguồn dữ liệu version (cập nhật theo API_Docs.txt 2026-11)

### 6.1 WEAO (nguồn chính)

**Bắt buộc**: header `User-Agent: WEAO-3PService` khi gọi mọi endpoint (xãp cả khi bị 429).

Domain: chính `weao.xyz`, `whatexpsare.online`; thay thế `whatexploitsaretra.sh`, `weao.gg`.

| Endpoint | Dữ liệu |
|---|---|
| `GET /api/versions/current` | Windows/Mac (+Android/iOS dạng `2.690.721`) hiện tại + `{c}Date` |
| `GET /api/versions/future` | Windows/Mac **sắp tới** — mới có trong bản update, Verlox dùng cho chip "sắp tới" |
| `GET /api/versions/past` | Windows/Mac trước đó (dùng để downgrade) |

Response chứa cả `{c}Response` metadata — Verlox bỏ qua. Khi **rate limit 429** WEAO trả `{"error":"Too Many Requests","rateLimitInfo":{...}}` → deserialize fail → code tự rơi về fallback; UI không nên spam nút Làm mới (bước 4: thêm cache TTL ~5 phút).

WEAO còn có **exploit tracker** (`/api/status/exploits/...`) trong API_Docs.txt — **ngoài phạm vi Verlox**, bỏ qua.

### 6.2 API chính thức Roblox (fallback)

- `https://clientsettingscdn.roblox.com/v2/client-version/WindowsPlayer` → JSON có `clientVersionUpload` (hash), `version` (số xyz).
- `.../WindowsStudio` → tương tự cho Studio (chính là nguồn `studioLatest` — WEAO không cung cấp current cho Studio rõ ràng).

### 6.3 Fallback khi WEAO past không dùng được

`DeployHistory.txt` trên CDN — quét các token `version-*` hợp lệ, dedupe, tối đa 30. Rust: `versions::fetch_deploy_history`.

## 7. Lộ trình chi tiết

### Bước 2 — Engine download (quan trọng nhất) — HOÀN TẤT

Port `InstallWorker` (backup/RobloxVersionDownloader1.2.py dòng ~296-612) thành `cdn.rs` + `extract.rs`:

1. ~~`cdn.rs::probe_cdns(hash)`~~ ✅ HEAD probe 5 endpoint song song (`join_all`), chọn latency thấp nhất; endpoint nào cũng fail thì về primary. Client riêng cho download: connect-timeout 12s, read-timeout 60s (không tổng timeout vì stream zip lớn).
2. ~~`fetch_manifest(cdn, hash)`~~ ✅ → `manifest::parse_manifest` (đã có).
3. Vòng lặp từng zip ✅: check cache (`<hash>-<zip>` MD5 bằng crate `md-5`) → nếu miss thì stream bằng `reqwest` `bytes_stream` vào `<zip>.part`, **resume bằng header `Range: bytes=N-`** (lỗi 416 → xóa .part tải lại từ đầu; server trả 200 thay vì 206 → cũng reset về đầu), emit `download://progress` mỗi ~0.4s; verify MD5 xong mới rename `.part` → đích.
4. `extract.rs` ✅: crate `zip` giải nén theo `PACKAGE_MAP` (zip lạ → giải nén vào gốc, như bản Python), `WebView2RuntimeInstaller.zip` → chạy exe `/silent /install` timeout 120s (tắt option thì bỏ qua), ghi `AppSettings.xml`.
5. `start_download` ✅: thay stub bằng `tauri::async_runtime::spawn` + `install_task`/`install_inner` trong `lib.rs`; state `DownloadState { cancel: Arc<AtomicBool>, running: AtomicBool }` qua `.manage()`; xong gọi `history::push`; ghi đè thư mục tồn tại: backend trả `EXISTS::<dir>`, UI confirm rồi gọi lại với `options.overwrite = true` (option `registerProtocol` tạm chỉ log cảnh báo — chờ bước 3).
6. Frontend ✅: `listen("download://progress")` vẽ 2 progress bar (overall + per-pkg) + speed/ETA + nút Hủy tải, card "Tiến trình tải" trong `index.html`.

Model tải tuần tự từng zip vẫn NHANH hơn bản Python vì stream + zero-copy; song song nhiều zip chỉ làm khi đã ổn.

### Bước 3 — Windows integration

- `win.rs::register_protocol` bằng crate `windows-registry` (đã có trong Cargo.toml): tạo `HKCU\Software\Classes\roblox` + `roblox-player`, giá trị `URL:<proto> Protocol`, `URL Protocol=""`, `shell\open\command = "<đường dẫn exe>" "%1"`.
- Nút gỡ protocol, checkbox option trong UI (module `register_protocol` command thay stub).
- Tạo shortcut đã xong từ bước 1.

### Bước 4 — UI hoàn thiện + query policy

- Progress kép + log đã có khung; thêm options panel (protocol / webview2 / shortcut / install dir).
- Browse thư mục qua `tauri-plugin-dialog` (thêm dep + permission).
- So sánh live vs installed → badge "LATEST / CŨ / CÓ SẮP CẬP NHẬT" trên từng row.
- Cache TTL 5 phút cho `get_live_versions`, chống 429 WEAO.
- Import legacy history bằng file picker thay `window.prompt`.

### Bước 5 — Build & phân phối

- `npm run build` → NSIS (`tauri.conf.json` đã set target `nsis`).
- Icon set đầy đủ: cần PNG 1024×1024 nguồn, chạy `npm run tauri icon <png>` để sinh `.ico/.png/.icns`.
- Harden CSP (đang `null`), bật devTools off ở release, kiểm tra `rustc` warning = 0.

## 8. Mapping legacy → dự án mới

| Bản Python (backup/RobloxVersionDownloader1.2.py) | Verlox |
|---|---|
| `validate_hash` (dòng 128) | `hash.rs::validate_hash` |
| `parse_manifest` (dòng 270) | `manifest.rs::parse_manifest` |
| `PACKAGE_MAP`, `APP_SETTINGS_XML` (dòng 55-86) | `manifest.rs` |
| `VersionFetchWorker` (dòng 929-997) | `versions.rs` (+ thêm endpoint future) |
| `load_history/save_history` (dòng 157-173) | `history.rs` (dùng `%LOCALAPPDATA%\Verlox`) |
| `is_unsafe_dir` (dòng 141-155) | `installed.rs::is_unsafe_dir` |
| `InstallWorker._main/_pick_cdn/_download_pkg/_extract` (dòng 296-586) | `cdn.rs` (probe/fetch/download_pkg) + `extract.rs` + `install_task`/`install_inner` (`lib.rs`) — đã port bước 2; `create_shortcut` option dùng lại `make_shortcut` của lệnh `create_shortcut` |
| `create_desktop_shortcut` (dòng 184) | `create_shortcut` command (PowerShell COM) |
| `_register_protocol` (dòng 588) | **bước 3**: `win.rs::register_protocol` |
| `HistoryRow` + UI PySide6 | `src/index.html` + `styles.css` + `main.js` |
| `PALETTE` (dòng 88) | `:root` CSS variables trong `styles.css` |
| `MusicPlayer` (pygame, dòng 216) | **ĐÃ BỎ** — không port (nếu muốn: `<audio>` trong webview, bước 4) |
| `main()`, PyInstaller (`lenhbuild.txt`) | `tauri build` tự lo |

## 9. Quy ước code

- Rust: `rustfmt` mặc định; hàm trả `Result<T, String>` với message tiếng Việt hiển thị trực tiếp lên UI; logic thuần (`hash`, `manifest`, `installed`, `history`) phải có `#[cfg(test)]`.
- Frontend: không framework, không bundler; giữ `window.__TAURI__` global API; đặtScreenState trong `state` object duy nhất (`main.js`).
- Serde struct nào expose về JS phải `#[serde(rename_all = "camelCase")]`.
- Command mới: khai báo trong `generate_handler![...]` + cập nhật bảng commands ở mục 4 của file này.
- Chỉ dẫn Wire khi build release (`cargo` performed DA XONG ở chế độ dev).
- Không chỉnh sửa bất cứ gì trong `backup/`.
- Không commend những logic quá rõ ràng và phổ quát.
- Không sử dụng emoji trong UI, tuyệt đối cấm emoji. Hãy sử dụng icon từ kho mã nguồn mở như Lucide hoặc Tabler.

## 10. Ghi chú vận hành

- Đổi tên repo `RobloxVersionDownloader` → `verlox`: chỉ đổi tên thư mục, không ảnh hưởng gì (`identifier` Tauri đã là `com.hoanglong.verlox`).
- Chưa khởi tạo git — nên `git init` + commit scaffold này làm baseline trước khi bắt tay bước 2 (`.gitignore` đã có sẵn).
- `API_Docs.txt` ở root là tài liệu WEAO gốc bản update 2026-11 — đối chiếu trước khi thêm field mới của WEAO.
