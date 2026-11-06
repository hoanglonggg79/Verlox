# VERLOX — Roblox Version Launcher

> Tài liệu dự án, viết cho người mới tiếp nhận (onboarding). Trạng thái tài liệu: **sau bước 3 + rewrite UI React-Vite + executor (WEAO + Umi ngoài API) + cache TTL 5 phút; đã gỡ DeployHistory + Lịch sử download**, cập nhật 2026-11-06.

---

## 1. Verlox là gì

Launcher & version manager cho Roblox Player/Studio trên Windows:

- Xem hash **version trực tiếp** (mới nhất / trước đó / sắp tới) từ WEAO + API chính thức của Roblox.
- **Sao chép hash** mới nhất, tải version bất kỳ bằng **custom hash**.
- **Tải theo executor** (đã triển khai, mô tả API ở mục 6.4): chọn executor (WEAO tracker + ngoài tracker như Umi, mục 6.5) → app tải đúng version Roblox mà executor đó hỗ trợ.
- Quét **các version đã cài trên máy** (`%LOCALAPPDATA%\Roblox\Versions`), chạy / mở thư mục / xóa từng version.
- Cache download + MD5 verify + resume (`.part`), giống bản Python cũ.
- Cache TTL 5 phút cho mọi query WEAO/API chính thức (`get_live_versions`, `list_executors`, `get_umi_executor`) — chống 429; nút Làm mới gọi với `force: true` bỏ qua TTL.

Tiền thân: **Roblox Version Downloader 1.2** (Python + PySide6) — toàn bộ nằm trong `backup/`, chỉ để tham khảo.

## 2. Stack & môi trường

| Thành phần | Vai trò |
|---|---|
| Rust stable (MSVC) + Cargo | Backend — mọi logic download/thư mục/registry |
| Tauri 2 | Khung desktop (webview WebView2, IPC commands + events) |
| React 19 + Vite 8 + lucide-react | Frontend trong `src/`, bundle ra `dist/`; IPC qua package npm `@tauri-apps/api` (core + event). Icon dùng Lucide (inline SVG, offline). `withGlobalTauri` đã tắt |
| Node.js + npm | Chạy Vite + `@tauri-apps/cli` |

Lệnh (npm run dev / build giữ nguyên như cũ):

```powershell
npm install                 # 1 lần, cài toàn bộ dev deps
npm run dev                 # chạy app: tauri dev → tự chạy `npm run ui:dev` (vite port 5173 strictPort)
npm run build               # build NSIS: tauri build → tự chạy `npm run ui:build` (vite build → dist/)
npm run ui:dev              # chỉ chạy Vite dev server (fe dev, không app)
npm run ui:build            # chỉ build frontend
cargo test                  # trong src-tauri — test logic thuần (hash, manifest, unsafe-dir, cdn, extract, win, cache TTL, guard)
cargo check                 # trong src-tauri — verify compile nhanh
```

`devUrl` = `http://localhost:5173`, `frontendDist` = `../dist` (đã đổi từ `../src` khi migrate React-Vite).

Môi trường đã verify trên máy này (2026-11-06): Node v24.19.0, npm 11.17.0, rustc 1.98.0, cargo 1.98.0. `cargo check` pass 0 warning, `cargo test` 38/38 pass (cập nhật sau session 2026-11-06 (3)+(4): Umi, cache TTL, guard xóa, gỡ history/DeployHistory).

Yêu cầu thêm cho máy khác: **VS Build Tools (MSVC linker)** khi build release, **WebView2 Runtime** (Windows 10/11 có sẵn).

## 3. Cấu trúc thư mục

```
verlox/
├── docs.md                # file này — đọc trước khi code
├── New_UI.md              # brief thiết kế UI mới (hệ chuyển động, contract DOM, checklist)
├── package.json           # npm scripts + deps FE (react, vite, lucide-react, @tauri-apps/api)
├── vite.config.mjs        # cấu hình Vite (port 5173 strictPort, build target chrome110 → dist/)
├── index.html             # entry Vite (root), mount #root
├── assets/                # icon executor (png/webp) — map theo tên executor đã normalize, dùng cho preview tab Roblox
├── index.html             # entry Vite (root), mount #root
├── API_Docs.txt           # tham chiếu API WEAO (bản update 2026-11, có cả endpoint /future)
├── License                # MIT
├── icon.ico               # icon gốc (bản sao nằm trong src-tauri/icons/)
├── src/                   # FRONTEND — React 19 (JSX, không TS)
│   ├── main.jsx           # createRoot → <App/>
│   ├── App.jsx            # toàn bộ state + invoke/listen + distribution props
│   ├── styles.css         # design system: sidebar 20/80, palette hướng A, motion tokens, keyframes, log terminal
│   ├── lib/
│   │   ├── tauri.js       # hasTauri + wrapper invoke/listen (@tauri-apps/api)
│   │   ├── utils.js       # fmtBytes/fmtSpeed/fmtEta/copyText
│   │   ├── hash.js        # normalizeHash (validate nhẹ phía UI)
│   │   ├── links.js       # DEV_DISCORD_URL / DEV_YOUTUBE_URL / WEAO_URL / REPO_URL
│   │   └── executorIcons.js   # map tên executor → assets/ (glob + alias + prefix fallback)
│   └── components/
│       ├── Sidebar.jsx          # logo giữa + 3 tab (Home/Roblox/Logs)
│       ├── HomeScreen.jsx       # mô tả app + link dev (opener) + "API Powered by WEAO"
│       ├── RobloxScreen.jsx     # Binary Type + Version Hash + Chọn Executor + preview + nút tải + progress + Installed
│       ├── ExecutorPreview.jsx  # icon + info + badge (ĐÃ CẬP NHẬT/CHƯA UPDATE/DETECTED) + banner downgrade warning
│       ├── InstalledCard.jsx   # scan máy + row action (chạy/protocol/shortcut/folder/xóa arm) + skeleton
│       ├── LogsScreen.jsx      # full-height log
│       └── LogPanel.jsx       # panel terminal (label chèn border, scanline, block cursor)
├── dist/                  # build output của Vite (tauri.conf frontendDist trỏ сюда) — gitignored
├── src-tauri/
│   ├── Cargo.toml         # deps: tauri 2, reqwest (rustls), zip, md-5, windows-registry...
│   ├── tauri.conf.json    # productName Verlox, frontendDist ../dist, devUrl :5173, withGlobalTauri false, csp null (bước 5 harden)
│   ├── capabilities/default.json   # ACL Tauri 2 (core:default)
│   ├── icons/icon.ico
│   └── src/
│       ├── main.rs        # entry, gọi verlox_lib::run()
│       ├── lib.rs        # toàn bộ #[tauri::command] + menu commands
│       ├── hash.rs       # validate_hash — đã port + test
│       ├── manifest.rs   # parse_manifest + PACKAGE_MAP + APP_SETTINGS_XML — đã port + test
│       ├── versions.rs   # WEAO (current/future/past) + API chính thức + filter exploits + Umi — đã port
│       ├── installed.rs  # scan Versions/, guard thư mục unsafe — đã port + test
│       ├── cdn.rs       # probe CDN + download resume + MD5 + progress — đã port + test (bước 2)
│       ├── extract.rs   # giải nén zip theo PACKAGE_MAP + WebView2 installer + AppSettings.xml — đã port + test (bước 2)
│       └── win.rs        # register/unregister protocol roblox-player:// (HKCU, marker "verlox") — đã port + test (bước 3)
└── backup/                # DỰ ÁN PYTHON CŨ — chỉ tham khảo, KHÔNG build/edit
    ├── RobloxVersionDownloader1.2.py   # nguồn port chính (~1660 dòng, có sẵn mọi logic cần)
    ├── README.md                      # mô tả bản 1.2
    ├── download_history.json          # dữ liệu lịch sử cũ (tính năng import vào app ĐÃ GỠ 2026-11-06)
    └── CleanerCache/CleanRobloxCache.bat
```

Frontend giữ nguyên tập ID DOM của bản cũ (mục 7 New_UI.md) để dễ đối chiếu; React quản lý state thay vì query DOM.

## 4. Kiến trúc runtime

Frontend gọi backend qua `invoke("<command>", {args-camelCase})`; backend trả JSON. Structs Rust serialize `snake_case` field + `#[serde(rename_all = "camelCase")]` → JS đọc camelCase (`studioLatest`, `sizeBytes`...). Command args JS dùng camelCase (`withSizes`, `installDir`, `filePath`) — Tauri 2 tự map sang snake_case của Rust.

Bảng commands hiện có (`src-tauri/src/lib.rs`):

| Command | Trạng thái | Ý nghĩa |
|---|---|---|
| `get_live_versions(force)` | ✅ chạy được | WEAO current/past/future + API chính thức → `{latest, studioLatest, futureWindows, previousWindows, previousMac}`. **Cache TTL 5 phút** (lib.rs `fetch_cached`); `force=true` bỏ qua TTL; khi fetch lỗi và có cache cũ (kể cả hết hạn) → trả stale (trừ khi `force` — force muốn dữ liệu mới hoặc lỗi rõ ràng) |
| `list_executors(force)` | ✅ chạy được (bước 4, 2026-11-06) | 1 request `GET weao.xyz/api/status/exploits` (UA WEAO-3PService) → lọc Windows + không hidden + `rbxversion` hợp lệ → `Vec<ExecutorInfo>` camelCase: `name, executorVersion, rbxVersion, updateStatus, detected, free, cost, keySystem`; 429 trả lỗi tiếng Việt. **Cache TTL 5 phút** như trên |
| `get_umi_executor(force)` | ✅ chạy được (2026-11-06) | Executor **Umi** ngoài danh sách WEAO (mục 6.5): đọc 2 file raw GitHub → `ExecutorInfo` duy nhất; `updateStatus` tự so hash Umi hỗ trợ với API chính thức Roblox. **Cache TTL 5 phút** như trên |
| `list_installed_versions(withSizes)` | ✅ chạy được | Scan `%LOCALAPPDATA%\Roblox\Versions` → hash, path, có Player/Studio exe, dung lượng |
| `launch_version(hash)` | ✅ chạy được | Spawn `RobloxPlayerBeta.exe` (working dir = thư mục version) |
| `open_folder(path)` | ✅ chạy được | Mở Explorer |
| `delete_version(hash)` | ✅ chạy được | Xóa thư mục version (chỉ chấp nhận dir tên đúng format hash). **Guard 2026-11-06**: từ chối nếu thư mục trùng target của tác vụ tải đang chạy (`DownloadState.current_dir` so khớp chính xác đường dẫn) → yêu cầu hủy tải hoặc đợi xong |
| `clear_cache()` | ✅ chạy được | Dọn `%LOCALAPPDATA%\Roblox\Downloads`, trả về số byte giải phóng; trả lỗi nếu đang có tác vụ tải (chống xóa `.part` đang ghi) |
| `create_shortcut(hash)` | ✅ chạy được | Tạo `.lnk` trên Desktop qua PowerShell COM |
| `register_protocol(hash)` | ✅ chạy được (bước 3) | Ghi registry `HKCU\Software\Classes\roblox{,-player}` cho version theo hash: default `URL:<proto> Protocol`, `URL Protocol=""`, `shell\open\command = "<exe>" "%1"`, kèm giá trị marker `verlox=<đường dẫn exe>` |
| `unregister_protocol()` | ✅ chạy được (bước 3) | Gỡ đăng ký protocol — chỉ xóa key nào có marker `verlox` (không đụng đăng ký gốc của Roblox chính thức); chưa đăng ký → trả lỗi tiếng Việt |
| `start_download(hash, installDir, options)` | ✅ chạy được | Spawn task download: probe CDN → manifest → tải từng zip (cache MD5, resume `.part`, verify MD5) → giải nén theo PACKAGE_MAP → ghi AppSettings.xml → shortcut tùy chọn. `installDir` là thư mục gốc, version cài vào con `{hash}`. Trả lỗi `EXISTS::<dir>` khi thư mục đích đã có nội dung và `options.overwrite` chưa bật (UI hỏi người dùng rồi gọi lại với `overwrite: true`). Task chạy dưới `RunningGuard` (Drop) — tự reset cờ `running` + xóa `current_dir` kể cả khi panic |
| `cancel_download` | ✅ chạy được | Set cờ cancel — task dừng ở điểm kiểm tra kế tiếp, các file `.part` được giữ để resume lần sau |

Event dùng thật ở bước 2 (frontend `listen` từ `@tauri-apps/api/event`, đăng ký trong `App.jsx`): `download://log` `{message, level}` (level: `info/muted/warning/success/error`), `download://progress` `{overallPct, pkgName, pkgPct, speedBps, etaSecs}`, `download://done` `{installDir}` (`null` nếu bị hủy giữa chừng), `download://error` `{title, message}`.

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

(Lịch sử download `%LOCALAPPDATA%\Verlox\download_history.json` đã gỡ khỏi app 2026-11-06; file cũ còn trên máy người dùng là dữ liệu mồ côi, không đọc không ghi.)

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

Response chứa cả `{c}Response` metadata — Verlox bỏ qua. Khi **rate limit 429** WEAO trả `{"error":"Too Many Requests","rateLimitInfo":{...}}` → deserialize fail → code tự rơi về fallback. Chống spam request: **cache TTL 5 phút đã triển khai 2026-11-06** (mục 4, `force` để bypass).

WEAO còn có **exploit tracker** — dùng cho tính năng "Tải theo executor" (kế hoạch bước 4, mô tả đầy đủ API ở mục 6.4).

### 6.2 API chính thức Roblox (fallback)

- `https://clientsettingscdn.roblox.com/v2/client-version/WindowsPlayer` → JSON có `clientVersionUpload` (hash), `version` (số xyz).
- `.../WindowsStudio` → tương tự cho Studio (chính là nguồn `studioLatest` — WEAO không cung cấp current cho Studio rõ ràng).

### 6.3 ~~Fallback DeployHistory.txt~~ — ĐÃ BỎ (2026-11-06)

Card "Version cũ hơn" (nạp `DeployHistory.txt` → chọn 1 trong 30 hash gần đây) đã gỡ khỏi UI và backend (`get_history_versions` + `versions::fetch_deploy_history` xóa) theo yêu cầu chủ dự án — ít dùng, vướng diện tích. Hạ cấp version giờ chỉ còn 2 đường: nút "Tải bản trước đó" (WEAO past) hoặc dán hash thủ công. Nếu sau này cần lại: port lại từ bản Python `VersionFetchWorker` (backup dòng 929-997, nguồn `https://setup.rbxcdn.com/DeployHistory.txt`).

### 6.4 WEAO exploit tracker — tính năng "Tải theo executor" (KẾ HOẠCH, bước 4)

Chọn executor (Potassium, Xeno, Synapse Z...) → hiển thị đúng version Roblox mà executor đó đang hỗ trợ → bấm tải bằng đúng hash đó. Kịch bản gốc: executor đã update → hash mới; executor lỗi thời → tải đúng bản cũ (trong docs WEAO, Synapse Z có `updateStatus: false` kèm `rbxversion: version-b67e7cbe150a4c59` — bản cũ).

**Nguồn**: `GET https://weao.xyz/api/status/exploits` — 1 request trả toàn bộ 30+ executor.

- Gọi endpoint **tất cả**, không gọi per-executor `.../[exploit]`: slug `[exploit]` không được quy định trong docs (title có khoảng trắng như "Synapse Z", không biết normalize kiểu gì), và mỗi lần chọn lại 1 request sẽ nhanh hết hạn mức 429.
- Bắt buộc header `User-Agent: WEAO-3PService` — tái dùng helper `versions::weao()`.

**Field lấy từ response** (serde mặc định bỏ qua field lạ):

| Field API | Dùng làm | Ghi chú |
|---|---|---|
| `title` | Tên executor (dropdown) | |
| `version` | Version phần mềm của executor, hiển thị phụ | không nhầm với `rbxversion` |
| `rbxversion` | **Hash Roblox mà executor hỗ trợ — cốt lõi tính năng** | chạy qua `hash::validate_hash`; rỗng/thiếu → loại entry |
| `updateStatus` | Badge ĐÃ CẬP NHẬT / CHƯA UPDATE | bool |
| `detected` | Badge cảnh báo (bị Hyperion detect) | bool |
| `platform` | Chỉ giữ entry `"Windows"` | Verlox Windows-only |
| `free` / `cost` / `keysystem` | Thông tin phụ (Miễn phí / giá / key system) | |
| `uncStatus` / `uncPercentage` / `suncPercentage` | Thông tin phụ nếu UI cần | **không deserialize object `sunc`** (chứa suncScrap/suncKey) |
| `hidden` | `true` → loại khỏi danh sách | |
| `_id`, `roleId`, `index`, `extype`, `beta`... | Bỏ | field nội bộ của WEAO |

**Query policy**: **ĐÃ TRIỂN KHAI 2026-11-06** — `list_executors`/`get_umi_executor`/`get_live_versions` đều qua cache TTL 5 phút thống nhất (lib.rs `fetch_cached`, có test); nút Làm mới executor gọi với `force: true` (bỏ TTL), auto nạp khi vào tab dùng cache → vào lại tab trong 5 phút không tốn request WEAO. Fetch lỗi: fallback stale từ cache cũ (trừ `force` — force muốn mới hoặc lỗi rõ).

**Thiết kế khi triển khai** — ĐÃ TRIỂN KHAI 2026-11-06 (bước 4):

- Command `list_executors()` ✅ → `Vec<ExecutorInfo>` camelCase đúng thiết kế; filter thuần `versions::filter_exploits` có test (3 case: lọc platform/hidden/hash, map cost/keysystem, parse payload thật).
- UI ✅: tab Roblox — select "Chọn Executor" (auto nạp khi vào tab + nút refresh), preview icon (map tên executor → `assets/`, alias `Synapse Z→synz`, prefix fallback, thiếu → icon Bot) + hash hỗ trợ + badge (ĐÃ CẬP NHẬT / CHƯA UPDATE / DETECTED) + chip giá/key; chọn executor → 3 nút tải thay bằng 1 nút "Tải bản [Tên] hỗ trợ" tải đúng `rbxVersion` (reuse luồng EXISTS/progress); `updateStatus=false` → banner cảnh báo downgrade (nội dung cố định theo yêu cầu chủ dự án). WEAO: `keySystem` bool → chip "Key system"/"Không cần key"; `keySystem: null` (nguồn không ghi) → không hiển thị chip key; `free=false` + `cost=null` → không hiển thị chip giá.

### 6.5 Executor ngoài WEAO — Umi (ĐÃ TRIỂN KHAI 2026-11-06)

Umi không nằm trong danh sách API của WEAO — nguồn riêng trên GitHub raw:

| File raw | Dữ liệu |
|---|---|
| `NguoiGiaAnThitTreCon/umi/.../umi.txt` | Version phần mềm của Umi (hiển thị `v...` ở preview) |
| `NguoiGiaAnThitTreCon/umi/.../version.txt` | **Hash Roblox mà Umi hỗ trợ** — chạy qua `hash::validate_hash` |

- Command `get_umi_executor()` ✅ (đường dẫn const ở `versions.rs`): đọc 2 file raw → `umi_executor_from_texts` (hàm thuần, có test) → `ExecutorInfo{ name: "Umi", free: true, cost: null, keySystem: false }` — **Miễn phí + Không cần key** (xác nhận chủ dự án 2026-11-06), preview hiển thị 2 chip tương ứng.
- `updateStatus` của Umi **không dùng badge tĩnh**: backend tự so hash Umi hỗ trợ với `clientsettingscdn` (WindowsPlayer) từng lần nạp → hash khớp latest = ĐÃ CẬP NHẬT, khác = CHƯA UPDATE (kèm banner downgrade); API chính thức fail → mặc định đã cập nhật (tránh cảnh báo nhầm).
- Frontend (`App.jsx::loadExecutors`) nạp WEAO + Umi song song bằng `Promise.allSettled`: nguồn nào fail vẫn giữ nguồn kia (log lỗi từng nguồn), cả hai fail → pill lỗi; guard trùng tên — nếu WEAO sau này có executor "Umi" thì record WEAO thắng.
- Icon: `assets/umi.webp` — map tự động qua `executorIcons.js` (norm "umi" trùng tên file).

## 7. Lộ trình chi tiết

### Bước 2 — Engine download (quan trọng nhất) — HOÀN TẤT

Port `InstallWorker` (backup/RobloxVersionDownloader1.2.py dòng ~296-612) thành `cdn.rs` + `extract.rs`:

1. ~~`cdn.rs::probe_cdns(hash)`~~ ✅ HEAD probe 5 endpoint song song (`join_all`), chọn latency thấp nhất; endpoint nào cũng fail thì về primary. Client riêng cho download: connect-timeout 12s, read-timeout 60s (không tổng timeout vì stream zip lớn).
2. ~~`fetch_manifest(cdn, hash)`~~ ✅ → `manifest::parse_manifest` (đã có).
3. Vòng lặp từng zip ✅: check cache (`<hash>-<zip>` MD5 bằng crate `md-5`) → nếu miss thì stream bằng `reqwest` `bytes_stream` vào `<zip>.part`, **resume bằng header `Range: bytes=N-`** (lỗi 416 → xóa .part tải lại từ đầu; server trả 200 thay vì 206 → cũng reset về đầu), emit `download://progress` mỗi ~0.4s; verify MD5 xong mới rename `.part` → đích.
4. `extract.rs` ✅: crate `zip` giải nén theo `PACKAGE_MAP` (zip lạ → giải nén vào gốc, như bản Python), `WebView2RuntimeInstaller.zip` → chạy exe `/silent /install` timeout 120s (tắt option thì bỏ qua), ghi `AppSettings.xml`.
5. `start_download` ✅: thay stub bằng `tauri::async_runtime::spawn` + `install_task`/`install_inner` trong `lib.rs`; state `DownloadState { cancel: Arc<AtomicBool>, running: Arc<AtomicBool>, current_dir: Arc<Mutex<Option<PathBuf>>> }` qua `.manage()`, cờ `running` + `current_dir` do `RunningGuard` (Drop) tự reset khi task xong hoặc panic; ghi đè thư mục tồn tại: backend trả `EXISTS::<dir>`, UI confirm rồi gọi lại với `options.overwrite = true` (option `registerProtocol` giờ gọi `win.rs::register_protocol` — bước 3 đã hoàn tất).
6. Frontend ✅: `listen("download://progress")` vẽ 2 progress bar (overall + per-pkg) + speed/ETA + nút Hủy tải, card "Tiến trình tải" trong `index.html`.

Model tải tuần tự từng zip vẫn NHANH hơn bản Python vì stream + zero-copy; song song nhiều zip chỉ làm khi đã ổn.

### Bước 3 — Windows integration — HOÀN TẤT (2026-11-06)

- `win.rs` ✅: `register_protocol(exe)` port từ `_register_protocol` (Python dòng 588): crate `windows-registry`, tạo `HKCU\Software\Classes\roblox` + `roblox-player`, default `URL:<proto> Protocol`, `URL Protocol=""`, `shell\open\command = "<exe>" "%1"`. Thêm so với bản Python: giá trị marker `verlox=<exe>` để biết Verlox là người đăng ký.
- `unregister_protocol()` ✅: xóa 2 key trên **chỉ khi có marker `verlox`** — tránh phá đăng ký gốc của Roblox chính thức khi người dùng bấm gỡ mà Verlox chưa từng đăng ký; không có gì để gỡ → trả lỗi tiếng Việt lên log.
- Command `register_protocol(hash)` (lib.rs) ✅ thay stub: đăng ký cho version đã cài theo hash, tương tự `create_shortcut`. Option `registerProtocol` trong `start_download` giờ gọi thẳng `win.rs` sau khi giải nén xong (không còn cảnh báo "chưa port").
- UI ✅: checkbox "Đăng ký protocol roblox-player://" trong card tải (đọc vào options mỗi lần `start_download`), nút "Gỡ protocol" ở card "Đang có trên máy", nút "Protocol" trên từng row version có Player.
- Tạo shortcut đã xong từ bước 1.

### Bước 4 — UI hoàn thiện + query policy

- **UI rewrite 2026-11-06 HOÀN TẤT**: frontend chuyển sang React 19 + Vite 8 theo New_UI.md (khung hướng A Launchpad + mật độ hàng của B + panel log chất liệu terminal của D), icon lucide-react, motion tokens + skeleton + progress `scaleX` + sheen + arm-delete + full ID contract. Lệnh `npm run dev` / `npm run build` giữ nguyên.
- **Sidebar 20/80 2026-11-06 HOÀN TẤT**: layout mới — sidebar trái 20% (logo VERLOX giữa, block cursor) + 3 tab: **Home** (mô tả app, link Discord/YouTube/repo — mở qua `tauri-plugin-opener`, "API Powered by WEAO"), **Roblox** (Binary Type [Player/Studio] + Version Hash optional + Chọn Executor + 3 nút latest/previous/specified hoặc 1 nút executor, options, progress, installed), **Logs** (toàn bộ log full-height). Bỏ live-dot nhấp nháy + watermark "by HoangLong". `REPO_URL` đã điền trong `src/lib/links.js`.
- Executor feature ✅ (xem mục 6.4 + bảng commands); nút Previous chỉ bật cho Player (WEAO không có past Studio), nút trong hero disabled khi thiếu dữ liệu; nút "Tải bản mới nhất" dùng `liveBusy` — disable + title "Đang nạp..." khi live đang tải và chưa có `latest` (2026-11-06 (4)).
- Options panel ✅ (protocol / webview2 / shortcut); nút Shortcut từng row dùng lại command `create_shortcut`.
- Browse thư mục cài đặt: **cần thêm `tauri-plugin-dialog`** (dep + permission) — ở root hiện đang dùng `window.prompt/confirm` làm thay; link ngoài dùng `tauri-plugin-opener` (đã thêm dep + `opener:default` capability).
- So sánh live vs installed → badge trên từng row — chưa làm.
- **Cache TTL 5 phút CHO `get_live_versions` + `list_executors` + `get_umi_executor` — ĐÃ LÀM 2026-11-06 (4)**: `fetch_cached` trong lib.rs (TTL 300s, stale-on-error cho lệnh ngầm, `force` bypass — nút Làm mới executor dùng `force`), có test riêng.
- **GỠ card DeployHistory + Lịch sử download — 2026-11-06 (4)** theo yêu cầu chủ dự án (ít dùng, vướng diện tích): xóa 2 component + props/state/callback liên quan trong App.jsx, xóa 3 command (`get_history_versions`/`get_download_history`/`import_legacy_history`), xóa module `history.rs` + `versions::fetch_deploy_history` + dep `chrono`; card "Trên máy" giờ full-width. Chi tiết ở mục 6.3 + 5.6. Import legacy history bằng file picker cũng gỡ luôn theo tính năng (đã không còn UI).

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
| `load_history/save_history` (dòng 157-173) | **ĐÃ BỎ** (gỡ tính năng lịch sử 2026-11-06, module `history.rs` xóa) |
| `is_unsafe_dir` (dòng 141-155) | `installed.rs::is_unsafe_dir` |
| `InstallWorker._main/_pick_cdn/_download_pkg/_extract` (dòng 296-586) | `cdn.rs` (probe/fetch/download_pkg) + `extract.rs` + `install_task`/`install_inner` (`lib.rs`) — đã port bước 2; `create_shortcut` option dùng lại `make_shortcut` của lệnh `create_shortcut` |
| `create_desktop_shortcut` (dòng 184) | `create_shortcut` command (PowerShell COM) |
| `_register_protocol` (dòng 588) | `win.rs::register_protocol` — đã port (bước 3) |
| `HistoryRow` + UI PySide6 | **ĐÃ BỎ** (gỡ card lịch sử + DeployHistory 2026-11-06 theo yêu cầu chủ dự án) |
| `PALETTE` (dòng 88) | `:root` CSS variables trong `styles.css` |
| `MusicPlayer` (pygame, dòng 216) | **ĐÃ BỎ** — không port (nếu muốn: `<audio>` trong webview, bước 4) |
| `main()`, PyInstaller (`lenhbuild.txt`) | `tauri build` tự lo |

## 9. Quy ước code

- Rust: `rustfmt` mặc định; hàm trả `Result<T, String>` với message tiếng Việt hiển thị trực tiếp lên UI; logic thuần (`hash`, `manifest`, `installed`, `cdn` open_part, `extract` sanitize, `win`, `fetch_cached`, `filter_exploits`, `umi_executor_from_texts`, `RunningGuard`) phải có `#[cfg(test)]`.
- Frontend (từ 2026-11-06): React 19 + Vite 8, component function JSX trong `src/components/`; state tập trung trong `App.jsx` (useCallback/hooks), không thêm state library; IPC qua `src/lib/tauri.js` (`@tauri-apps/api` — `withGlobalTauri` đã tắt); giữ nguyên tập ID DOM cũ (mục 7 New_UI.md) khi sửa component.
- Icon chỉ dùng `lucide-react` (inline SVG khi bundle, offline). Vô hiệu mọi hiệu ứng qua `prefers-reduced-motion` (đã có trong `styles.css`).
- Serde struct nào expose về JS phải `#[serde(rename_all = "camelCase")]`.
- Command mới: khai báo trong `generate_handler![...]` + cập nhật bảng commands ở mục 4 của file này.
- Chỉ dẫn Wire khi build release (`cargo` performed DA XONG ở chế độ dev).
- Không chỉnh sửa bất cứ gì trong `backup/`.
- Không commend những logic quá rõ ràng và phổ quát.
- Không sử dụng emoji trong UI, tuyệt đối cấm emoji. Icon từ Lucide (qua package `lucide-react`, tương đương Tabler/Lucide sprite trong New_UI.md).

## 10. Ghi chú vận hành

- Đổi tên repo `RobloxVersionDownloader` → `verlox`: chỉ đổi tên thư mục, không ảnh hưởng gì (`identifier` Tauri đã là `com.hoanglong.verlox`).
- Git đã khởi tạo ngày 2026-11-06 (nhánh `main`): baseline commit chụp trạng thái "sau bước 2", sau đó commit vá 3 lỗi logic (clear_cache đếm size trước khi xóa / probe CDN chỉ chấp nhận HTTP 200 / RunningGuard reset cờ khi panic). Quy ước: commit sau mỗi tính năng hoàn chỉnh và ổn định.
- 2026-11-06: commit bước 4 (UI rewrite React + executor pill). Phát hiện + vá lỗi `download_pkg` mở `.part` thiếu `write(true)` ở nhánh tải mới — Windows std từ chối "creating or truncating a file requires write or append access", khiến **mọi** lần tải mới fail từ bước 2 (đường chạy thật không có test). Đã tách helper `cdn::open_part` + test hồi quy cho cả nhánh fresh lẫn resume.
- 2026-11-06 (2): lỗi giải nén `content-*.zip` — zip của Roblox đóng gói kiểu Windows cũ: entry dùng `\` làm separator và có entry gốc tên chỉ là `\`; `zip::ZipArchive::extract()` từ chối với "invalid Zip archive: Invalid file path" (Python `extractall` thì sanitize nên bản 1.2 chạy được). Đã thay bằng vòng giải nén tự viết trong `extract::extract_zip` + `sanitize_entry_name` đúng ngữ nghĩa `zipfile.extractall` (bỏ `''`/`.`/`..`, strip drive + leading slash, bỏ qua entry rác, NUL → skip) — an toàn zip-slip vì mọi path đều nằm trong out_dir; test hồi quy tái tạo zip kiểu Roblox + đã verify trực tiếp với file `content-avatar.zip` thật trong cache (73 file).
- `API_Docs.txt` ở root là tài liệu WEAO gốc bản update 2026-11 — đối chiếu trước khi thêm field mới của WEAO.
- 2026-11-06 (3): thêm executor **Umi** ngoài WEAO (command `get_umi_executor`, mục 6.5) + badge DETECTED trong preview (trước đó docs ghi nhưng code thiếu). Vá hiệu ứng progress: `.progress-fill` transition 120ms trong khi backend emit `download://progress` chu kỳ 0.4s → bar chạy nửa chừng đứng im (giật); sửa thành `400ms linear` khớp nhịp emit → chuyển động liên tục; keyframe `sheen` thiếu `from` nên dải sáng nhảy pop giữa bar mỗi vòng loop — thêm `from: translateX(-100%)`. `ExecutorInfo.keySystem` đổi từ `bool` sang `Option<bool>` (null = không biết, không hiển thị chip). Commit `47481f4`.
- 2026-11-06 (4): (a) guard `delete_version` — từ chối xóa đúng thư mục đang là target của tác vụ tải (`DownloadState.current_dir`, set khi task spawn, clear bởi `RunningGuard::drop` kể cả panic, có test); (b) nút "Tải bản mới nhất" wire `liveBusy` (disable + title khi đang nạp và chưa có `latest`); (c) cache TTL 5 phút `fetch_cached` cho `get_live_versions`/`list_executors`/`get_umi_executor` (tham số `force` bypass — nút Làm mới executor; stale-on-error cho lệnh ngầm, không stale khi force, có 4 test); (d) Umi hiển thị **Miễn phí + Không cần key** (`free: true`, `keySystem: Some(false)` — xác nhận chủ dự án); (e) gỡ card DeployHistory + Lịch sử download (UI + props + 3 command + `history.rs` + `fetch_deploy_history` + dep `chrono`), card "Trên máy" full-width. `cargo test` 38/38, `cargo check` 0 warning, `vite build` pass. Chưa commit — đợi yêu cầu.
