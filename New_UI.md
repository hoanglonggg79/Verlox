# New_UI.md — Brief thiết kế lại GUI Verlox

> Cập nhật 2026-11-06 (lần 2): chủ dự án đổi layout — bỏ khung cuộn dọc 1 cột,
> chuyển sang **sidebar trái 20% / content 80%** với 3 tab (Home / Roblox /
> Logs), logo VERLOX căn giữa trong sidebar, bỏ live-dot và watermark. Các
> hệ thống motion (mục 3), icon (mục 6) và event contract giữ nguyên; panel
> log chỉ còn trên tab Logs.
>
> Tài liệu này dành cho team UI. Mục tiêu: thay thế giao diện port từ PySide6
> (bản 1.2) bằng một UI hiện đại, có hiệu ứng động, vẫn chạy trong Tauri 2
> (WebView2, Windows-only). Không cần đụng backend Rust.

---

## 1. Vì sao UI hiện tại cảm giác cũ

- **Palette mang đúng PySide6 bản 1.2**: nền tím than `#0d0d1a` + đỏ `#e8314a`,
  border phẳng 1px, radius 8px, không có chiều sâu (không glow, không gradient,
  không shadow) — nhìn như "admin tool", không phải sản phẩm.
- **Typography đơn điệu**: Consolas dùng cho gần như mọi thứ (brand, nút, hash,
  log) — thiếu phân cấp thị giác.
- **Không icon** (project cấm emoji mà chưa có icon thay thế) → mọi hành động
  là nút chữ.
- **Layout cuộn dọc 6 card đều nhau**, không có "điểm hero" — người dùng không
  biết nhìn vào đâu trước.
- **Chuyển động gần như không có**: chỉ `transition 0.15s` trên nút và đổi
  `width` progress bar. Không entrance, không feedback, không trạng thái loading.

## 2. Ràng buộc kỹ thuật (BẮT BUỘC — đọc trước khi phác)

> Cập nhật 2026-11-06: ràng buộc số 1 được chủ dự án thay đổi — UI mới dùng
> React 19 + Vite 8 (đã triển khai). Các ràng buộc còn lại vẫn giữ nguyên.

1. **Stack**: ~~HTML/CSS/JS thuần~~ → React 19 + Vite 8, bundle ra `dist/`
   (`tauri.conf.json`: `frontendDist ../dist`, `devUrl :5173`). IPC qua package
   npm `@tauri-apps/api` (core + event), không còn `window.__TAURI__`
   (`withGlobalTauri` đã tắt). Icon qua `lucide-react` (inline SVG, offline).
2. **Chỉ chạy Windows + WebView2 (Chromium evergreen)**: dùng tự do
   `backdrop-filter`, `color-mix`, CSS variables, `@keyframes`, container query.
   Tránh API quá mới vì WebView2 trên máy người dùng cập nhật độc lập với app.
3. **Không load font/icon từ CDN lúc runtime**: app phải chạy offline và CSP
   sẽ được harden ở bước 5 (hiện đang `null`). Font (woff2) và icon phải vendored
   vào `src/fonts/`, `src/icons.svg`.
4. **Cấm emoji tuyệt đối** — icon lấy từ kho nguồn mở MIT: **Tabler** hoặc
   **Lucide** (SVG inline, stroke 1.5–2px, chuẩn hóa một cỡ 16/18px).
5. **Giữ contract DOM với `src/main.js`** (danh sách ID/class ở mục 7). Đổi
   cấu trúc thì phải sửa `main.js` cùng lúc — không được phá kiểu camelCase của
   invoke/listen hiện có.
6. Window: 960×880, min 760×700; nội dung chính cuộn dọc; giữ dark theme
   (cả 4 hướng dưới đều là dark).
7. Quy ước hiện hành (docs.md mục 9): state đặt trong object `state` duy nhất
   của `main.js`; không thêm comment cho logic hiển nhiên.

## 3. Hệ thống motion dùng CHUNG (áp dụng cho mọi hướng)

### 3.1 Tokens

```css
:root {
  --ease-out:    cubic-bezier(0.22, 1, 0.36, 1);    /* easeOutQuint */
  --ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1); /* back-out nhẹ */
  --dur-fast: 120ms;
  --dur-med:  220ms;
  --dur-slow: 420ms;
}

@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```

### 3.2 Luật giữ 60fps

- Chỉ animate `transform` + `opacity` (và gradient overlay). **Không** animate
  `width/height/top/left` — progress bar dùng `transform: scaleX()` với
  `transform-origin: left` thay vì đổi width.
- Stagger vào list: mỗi row delay `calc(var(--i) * 40ms)`, tối đa ~12 phần tử.
- Hover: `translateY(-1px)` hoặc `scale(1.02)`; press: `scale(0.97)`; thời lượng
  `--dur-fast`.
- Glow vẽ bằng overlay `::after` và animate opacity (rẻ hơn animate box-shadow).
- Không dùng JS animation library. CSS là đủ; JS chỉ set `--i` và class trạng
  thái (`is-busy`, `is-done`...).

### 3.3 Hiệu ứng component chuẩn (bắt buộc ở mọi hướng)

| Component | Hiệu ứng | Gợi ý kỹ thuật |
|---|---|---|
| Row list (installed/history) | Slide-fade vào khi refresh, stagger theo `--i` | `@keyframes row-in { from { opacity:0; transform:translateY(6px); } }` |
| Progress bar | Track tối, fill `scaleX` + **sheen chạy qua** khi đang tải; xong đổi màu success và tắt sheen | sheen: gradient trắng ~.18, translateX(-100%→100%), vòng 1.6s |
| % tổng tiến trình | Hiển thị to, dùng `font-variant-numeric: tabular-nums` (không cần count-up bằng JS) | — |
| Live dot `#live-dot` | `data-state="ok"`: pulse ring lan ra mỗi 2s; `"bad"`: nhấp nháy đỏ 1s/vòng; idle: xám tĩnh | box-shadow từ 0 lan ra 8px rồi về transparent |
| Nút | hover lift + đổi tint; active scale .97; disabled giảm opacity .45 | — |
| Card | Mở app: fade-up cascade theo thứ tự section (mỗi card một `--i`) | — |
| Log line | Dòng mới fade-slide-in; giữ level màu hiện có (muted/info/ok/warn/error) | — |
| Download done | Burst nhỏ: icon check scale-in rồi mờ dần; card tiến trình co lại | co height bằng `grid-template-rows 0fr→1fr` |
| Copy hash | Icon copy → icon check trong 900ms ngay trên nút | — |
| Confirm xóa (arm) | "Xóa" → "Xóa thật?" đổi nền đỏ + shake đúng 1 nhịp | `@keyframes shake { 25% { translateX(-2px); } 75% { translateX(2px); } }` |

Skeleton loading: `list_installed_versions(withSizes=true)` tính size có thể
chậm — khi đang chờ thì vẽ 3 row skeleton (opacity pulse .45↔.75, mỗi row là
khối `span.skeleton` với width khác nhau).

---

## 4. Bốn hướng UI đề xuất

> Cả 4 đều giữ nguyên 7 section hiện có (live versions / hash tùy chỉnh /
> tiến trình / DeployHistory / installed / history / log) — chỉ khác cách tổ
> chức và chất liệu thị giác.

### Hướng A — "LAUNCHPAD" (game-launcher deck) — khuyến nghị chính

**Concept**: Verlox là launcher game → nhìn như launcher game hiện đại
(Xbox app / Discord / Steam): hero download nổi bật, glow đỏ giữ thương hiệu
Verlox, grid card phía dưới, trẻ trung mà không lòe loẹt.

```
+--------------------------------------------------------------+
| VERLOX ------------------------------------------ dot v2.0.0  |
+--------------------------------------------------------------+
| +----------------------------------------------------------+ |
| | HERO TẢI:  [input hash________]  [ TẢI BẢN NÀY ]         | |
| | hash mới nhất: version-eb46... [copy] [tải mới nhất]     | |
| | ▓▓▓▓▓▓▓▓░░░░ 62.4%  · 12.3 MB/s · ETA 01:42              | |
| +----------------------------------------------------------+ |
| + Phiên bản +   + Trên máy +   + Lịch sử +   + DeployHistory + |
| | live grid |   | rows       |  | rows     |  | select       | |
| +-----------+   +------------+  +----------+  +---------------+ |
| + Log (dải terminal nằm ngang dưới cùng)                   + |
+--------------------------------------------------------------+
```

- **Palette** (CSS variables):
  ```css
  --bg #0a0c10; --surface #12151c; --surface-2 #161a23;
  --accent #ff4655; --accent-2 #ff7a45;   /* CTA gradient đỏ→cam */
  --fg #f2f4f8; --muted #7d8590; --border rgb(255 255 255 / 0.07);
  --success #3ee88a; --warning #f5c842; --error #ff5566;
  ```
- **Chất liệu**: card phủ gradient rất nhẹ 145deg từ surface → surface-2,
  border trắng alpha thấp, radius 14px; CTA chính có glow
  `box-shadow: 0 8px 32px rgb(255 70 85 / .28)` và **thở chậm 3s/vòng khi có
  task đang chạy**; nền app có vignette đỏ alpha .04 ở góc trên.
- **Typography**: Inter Variable (`src/fonts/Inter.woff2`) cho UI; hash/log
  giữ mono nhưng đổi sang JetBrains Mono.
- **Motion riêng**: hero progress là "màn hình chính" — số % cỡ 34px; khi
  xong, hero flip sang trạng thái DONE (icon check xanh) rồi mới reset.
- **Khi nào chọn**: muốn sản phẩm "cool", đúng chất launcher, cảm giác thay
  đổi rõ nhất mà vẫn dễ làm. Glow chỉ dùng cho CTA chính + 1 accent, không
  glow cả trang.

### Hướng B — "PRO TOOL" (Linear / Vercel Geist)

**Concept**: công cụ nghiêm túc, gọn, mật độ thông tin cao — tối ưu thao
tác: bảng version dày, chuyển động tinh tế 120–200ms, không glow.

```
+--------------------------------------------------------------+
| VERLOX  [Live] [Trên máy] [Lịch sử] [Log]           dot v2.0.0|
|          ^ tab pills, underline trượt theo tab active          |
|  (content của tab active, max-width ~920px, căn giữa)         |
|  hash input + nút tải nằm dạng "command bar" phía trên content |
+--------------------------------------------------------------+
```

- **Palette**:
  ```css
  --bg #08090a; --panel #101113; --border #1f2328;
  --accent #6366f1; --fg #f7f8f8; --muted #8a8f98;
  --danger #e5484d; --success #46a758;
  ```
- **Chất liệu**: phẳng tuyệt đối, không gradient, radius 8–10px, phân cấp
  bằng khoảng trắng + cỡ chữ; row 1 dòng, hover nền `rgb(255 255 255 / .04)`;
  hairline divider giữa các nhóm.
- **Typography**: Inter 13px mặc định; heading 12px uppercase letter-spacing
  .08em màu muted; số/hash mono 12px.
- **Motion riêng**: underline tab trượt bằng `transform: translateX()`; tooltip
  fade 100ms; toast khi copy thay vì chỉ ghi log; focus ring accent 2px; skeleton
  xám tối giản.
- **Khi nào chọn**: ưu tiên tốc độ đọc thông tin, thao tác nhanh, khó lỗi thời.
  Rủi ro: thiếu "wow", giống dashboard doanh nghiệp.

### Hướng C — "FLUENT / MICA" (Windows 11 native)

**Concept**: nhìn như một phần của Windows — acrylic/mica mờ, bo góc Fluent,
theo system accent, cảm giác "premium native" trên Win 11.

```
| (custom titlebar trong suốt · drag-region · nút minimize/max/close) |
| +----------------------------------------------------------+    |
| | acrylic panel: content app bo 8px, cách viền 12px        |    |
| | sidebar trái (icon + label) ~200px | content bên phải     |    |
| +----------------------------------------------------------+    |
```

- **Palette**:
  ```css
  --bg: rgb(28 28 32 / .78); backdrop-filter: blur(28px) saturate(1.4);
  --card: rgb(43 43 48 / .66); --stroke: rgb(255 255 255 / .08);
  --accent: system accent màu Windows (registry AccentColor) — fallback #4cc2ff;
  --text #f3f3f3; --muted #9a9a9a;
  ```
- **Chất liệu**: phân lớp rõ (nền mờ → card đặc → control đặc nhất); hover
  "reveal" quét theo vị trí chuột bằng radial-gradient theo `--x --y`
  (JS cập nhật 2 biến khi `mousemove`).
- **Typography**: "Segoe UI Variable Text" (Win 11 có sẵn, không cần bundle
  font); mono dùng Cascadia Mono (có sẵn trên Windows).
- **Motion riêng**: chuẩn Fluent `cubic-bezier(0.16, 1, 0.3, 1)` 200–300ms;
  page transition fade-through.
- **Điểm lưu ý**: phải làm **custom titlebar** trong Tauri (`decorations: false`
  + `data-tauri-drag-region` + 3 nút cửa sổ) — đẹp nhất nhưng là hướng duy nhất
  đụng cấu hình Tauri và JS window control; không làm titlebar thì hiệu ứng
  mica sẽ không trọn vẹn.
- **Khi nào chọn**: muốn tinh tế "thuộc hệ điều hành". Nhược điểm: ít bản sắc
  riêng.

### Hướng D — "CYBER TERMINAL" (giữ gốc hacker, hiện đại hóa)

**Concept**: khoét sâu bản sắc sẵn có của Verlox (mono, đỏ, hash) — biến nó
thành một "terminal thời đại mới": lưới mờ, phosphor, boot log — nhấn đúng
chất "version hacker".

```
| VERLOX ▒▒▒ build 2.0.0 · online(dot) · verlox@local:~$              |
| ┌[LIVE]───────┐ ┌[TRÊN MÁY]───┐ ┌[LỊCH SỬ]────┐                     |
| │ live grid   │ │ rows + nút  │ │ rows        │  ← mỗi panel là    |
| └─────────────┘ └─────────────┘ └─────────────┘    "cửa sổ term"   |
| download: version-eb46... ▓▓▓▓▓░░ 62%  12.3MB/s  ETA 01:42          |
| log = boot stream, con trỏ block nhấp nháy 1s/vòng                  |
```

- **Palette**:
  ```css
  --bg #05070a;
  --grid: rgb(0 230 138 / .05);  /* nền lưới 32px bằng repeating-linear-gradient */
  --phosphor #00e68a; --accent #ff2e4d;
  --fg #d8ffe9; --muted #3f694f;
  ```
- **Chất liệu**: panel là "cửa sổ terminal" — label chèn lên border tại góc
  trên trái; hash sáng nhẹ `text-shadow: 0 0 12px rgb(0 230 138 / .35)`;
  1 scan-line rất nhẹ chạy dọc nền 8s/vòng — không lạm dụng.
- **Typography**: JetBrains Mono toàn trang — chủ động lấy toàn mono làm điểm
  mạnh thương hiệu, khác biệt hẳn các launcher khác.
- **Motion riêng**: dòng log cuối nhập kiểu "typed" nhanh (~60ms/ký tự, chỉ áp
  cho 1 dòng cuối cùng); progress dạng segmented blocks `▓▓▓░░░`; con trỏ block
  nhấp nháy; glitch nhẹ 200ms trên chữ VERLOX đúng 1 lần lúc mở app.
- **Khi nào chọn**: muốn khác biệt nhất, đúng chất công cụ kỹ thuật. Rủi ro:
  glow/hiệu ứng gây mỏi mắt và giảm khả năng đọc nếu contrast làm kém.

---

## 5. So sánh & khuyến nghị

| Tiêu chí | A Launchpad | B Pro tool | C Fluent | D Cyber |
|---|---|---|---|---|
| Cảm giác hiện đại | Cao | Trung | Cao | Cao (khác biệt) |
| Effort implement | Trung | Thấp | Cao (titlebar) | Trung |
| Khả năng lỗi thời | Trung | Thấp nhất | Theo Windows | Trung |
| Chất "launcher game" | Cao | Thấp | Trung | Trung |
| Chất "công cụ kỹ thuật" | Trung | Cao | Trung | Cao |

**Khuyến nghị**: lấy **A làm khung chính** (hero download + grid card + glow
CTA), **mượn của B** tab pill + mật độ hiển thị cho section "Trên máy", và
**mượn của D** chất liệu terminal cho riêng panel Log (mono, cửa sổ bo viền,
boot-style). Bộ ba này giữ chất game launcher mà vẫn đậm bản sắc kỹ thuật,
effort trung bình, không cần custom titlebar. C chỉ nên cân nhắc nếu sau này
muốn release một "Windows-native edition".

## 6. Icon bắt buộc phải có (Tabler/Lucide, inline SVG)

| Chỗ dùng | Icon gợi ý (Tabler) |
|---|---|
| Làm mới | `arrow-rotate` |
| Sao chép | `copy` → đổi `check` 900ms khi thành công |
| Tải | `download` · Hủy: `player-stop` |
| Chạy | `player-play` · Studio: `code` |
| Thư mục | `folder` |
| Xóa | `trash` (arm: nền đỏ + shake) |
| Protocol | `link` · Gỡ protocol: `link-off` |
| Dọn cache | `broom` |
| Lịch sử | `history` |
| Live dot | giữ dot CSS hiện có, thêm pulse |
| Brand | chữ VERLOX (+ block cursor `_` nhấp nháy nếu chọn D) |

Tổ chức: ~~1 file `src/icons.svg` dạng sprite~~ → đã dùng `lucide-react`
(component inline SVG khi bundle). Lưu ý ~~`btn()` trong `main.js`~~ →
button trong React nhận icon + label qua component `lucide-react`.

## 7. Contract DOM với main.js (KHÔNG được phá khi re-design)

> Cập nhật 2026-11-06: `main.js` đã được thay bằng React (`App.jsx` + components).
> Component render giữ nguyên toàn bộ ID/class dưới đây để dễ đối chiếu; logic
> truy cập DOM chuyển sang state/props React. Event & payload Tauri giữ nguyên.

`React` render các ID sau — bắt buộc tồn tại đúng tên:

- Nút: `#btn-refresh-live #btn-copy-latest #btn-download-latest
  #btn-refresh-installed #btn-unregister-protocol #btn-clear-cache
  #btn-load-history #btn-download-custom #btn-import-history
  #btn-cancel-download`
- Input: `#custom-hash #opt-register-protocol #history-select`
- Hiển thị: `#latest-player #latest-studio #prev-win #future-win #live-dot
  (data-state="ok|bad") #log #dl-hash #dl-overall-pct #dl-bar-overall
  #dl-pkg-name #dl-pkg-pct #dl-bar-pkg #dl-speed #dl-eta #hash-preview
  #installed-list #history-list`
- Class mà JS hiện đang thao tác: `.log-line` + level `muted/info/ok/warn/error`;
  `.row .empty .badge(.ok/.info) .danger (arm xóa) .invalid (hash sai)
  .idle (dl-hash rảnh)`.
- Row trong `#installed-list` / `#history-list` do JS render — nếu đổi class
  thì phải cập nhật `main.js` song song. **Progress bar**: nên chuyển cả hai
  bên sang `el.style.setProperty("--pct", v)` và CSS dùng
  `transform: scaleX(var(--pct))` (JS hiện set `style.width` trực tiếp).
- Event & payload giữ nguyên: `download://log {message, level}`;
  `download://progress {overallPct, pkgName, pkgPct, speedBps, etaSecs}`;
  `download://done {installDir}` (null = bị hủy);
  `download://error {title, message}` — map level đang có trong LOG_LEVEL_MAP.

## 8. Checklist bàn giao (team UI tự nghiệm)

- [ ] Chạy thật qua `npm run dev` từ thư mục gốc, mọi thao tác vẫn dùng được:
      refresh live, copy hash, tải thử 1 hash thật, hủy giữa chừng (kiểm tra
      progress + `.part` resume lần sau), nút Protocol / Gỡ protocol, quét
      installed, xóa 2 bước (arm), dọn cache, import legacy history.
- [ ] Không sửa gì trong `src-tauri/` và `backup/`.
- [ ] Animation giữ 60fps khi đang tải (progress + log chạy đồng thời) —
      kiểm tra bằng DevTools của WebView2 (chế độ dev bật sẵn).
- [ ] `prefers-reduced-motion: reduce` — mọi hiệu ứng tắt sạch.
- [ ] Không emoji ở bất kỳ đâu; icon chỉ từ Tabler/Lucide inline.
- [ ] Không fetch icon/font runtime ngoài — tất cả nằm trong `src/`.
- [ ] Test ở size window min 760×700: không vỡ layout, không 2 tầng scrollbar.
