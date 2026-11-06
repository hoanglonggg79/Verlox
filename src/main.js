const invoke = window.__TAURI__?.core?.invoke ?? null;
const listen = window.__TAURI__?.event?.listen ?? null;

const state = {
  live: null,
  armDelete: null,
  armTimer: null,
};

const LOG_LEVEL_MAP = {
  muted: "muted",
  warning: "warn",
  success: "ok",
  error: "error",
  info: "info",
};

const $ = (sel) => document.querySelector(sel);

function log(msg, kind = "info") {
  const el = $("#log");
  const line = document.createElement("div");
  line.className = `log-line ${kind}`;
  const ts = new Date().toLocaleTimeString("vi-VN", { hour12: false });
  line.textContent = `[${ts}] ${msg}`;
  el.appendChild(line);
  el.scrollTop = el.scrollHeight;
}

function fmtBytes(n) {
  if (!n) return "0 B";
  if (n < 1024) return `${n} B`;
  if (n < 1048576) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1073741824) return `${(n / 1048576).toFixed(2)} MB`;
  return `${(n / 1073741824).toFixed(2)} GB`;
}

function copyText(text) {
  if (navigator.clipboard?.writeText) {
    return navigator.clipboard.writeText(text);
  }
  return new Promise((resolve, reject) => {
    const ta = document.createElement("textarea");
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    try {
      document.execCommand("copy") ? resolve() : reject(new Error("copy failed"));
    } finally {
      ta.remove();
    }
  });
}

function btn(label, onClick) {
  const b = document.createElement("button");
  b.className = "btn small";
  b.textContent = label;
  b.addEventListener("click", onClick);
  return b;
}

function badge(text, kind) {
  const s = document.createElement("span");
  s.className = `badge ${kind}`;
  s.textContent = text;
  return s;
}

function emptyRow(text) {
  const d = document.createElement("div");
  d.className = "empty";
  d.textContent = text;
  return d;
}

async function refreshLive() {
  if (!invoke) return;
  $("#btn-refresh-live").disabled = true;
  try {
    const live = await invoke("get_live_versions");
    state.live = live;
    $("#latest-player").textContent = live.latest ?? "—";
    $("#latest-studio").textContent = live.studioLatest ?? "—";
    $("#prev-win").textContent = live.previousWindows ?? "—";
    $("#future-win").textContent = live.futureWindows ?? "—";
    $("#live-dot").dataset.state = "ok";
    log("Đã nạp thông tin phiên bản trực tiếp", "ok");
  } catch (err) {
    $("#live-dot").dataset.state = "bad";
    log(String(err), "error");
  } finally {
    $("#btn-refresh-live").disabled = false;
  }
}

async function copyLatest() {
  const h = state.live?.latest;
  if (!h) {
    log("Chưa có hash mới nhất — bấm Làm mới", "error");
    return;
  }
  try {
    await copyText(h);
    log(`Đã sao chép ${h}`, "ok");
  } catch {
    log("Sao chép thất bại", "error");
  }
}

function fmtSpeed(bps) {
  if (!bps || bps <= 0) return "";
  if (bps < 1024) return `${Math.round(bps)} B/s`;
  if (bps < 1048576) return `${(bps / 1024).toFixed(1)} KB/s`;
  return `${(bps / 1048576).toFixed(2)} MB/s`;
}

function fmtEta(secs) {
  if (!secs || secs <= 0) return "";
  const total = Math.round(secs);
  const m = String(Math.floor(total / 60)).padStart(2, "0");
  const s = String(total % 60).padStart(2, "0");
  return `ETA ${m}:${s}`;
}

function resetProgressUi() {
  $("#dl-overall-pct").textContent = "0.0%";
  $("#dl-pkg-pct").textContent = "0%";
  $("#dl-bar-overall").style.width = "0%";
  $("#dl-bar-pkg").style.width = "0%";
  $("#dl-pkg-name").textContent = "Gói";
  $("#dl-speed").textContent = "";
  $("#dl-eta").textContent = "";
}

function setDownloadBusy(busy) {
  $("#btn-download-latest").disabled = busy;
  $("#btn-download-custom").disabled = busy;
  $("#btn-cancel-download").disabled = !busy;
  if (!busy) {
    const hashEl = $("#dl-hash");
    hashEl.textContent = "Chưa có tác vụ tải nào";
    hashEl.classList.add("idle");
  }
}

async function downloadHash(hash, overwrite = false) {
  const options = {
    registerProtocol: false,
    installWebview2: false,
    createShortcut: false,
    overwrite,
  };
  try {
    await invoke("start_download", { hash, installDir: null, options });
    const hashEl = $("#dl-hash");
    hashEl.textContent = hash;
    hashEl.classList.remove("idle");
    resetProgressUi();
    setDownloadBusy(true);
    log(`Bắt đầu tải ${hash}`, "ok");
  } catch (err) {
    const msg = String(err);
    if (msg.startsWith("EXISTS::")) {
      const dir = msg.slice("EXISTS::".length);
      if (window.confirm(`Phiên bản đã được cài tại:\n${dir}\n\nCài đè (ghi đè lên thư mục cũ)?`)) {
        downloadHash(hash, true);
      } else {
        log("Đã hủy — thư mục cài đè đã tồn tại", "muted");
      }
    } else {
      log(msg, "error");
    }
  }
}

async function cancelDownload() {
  try {
    await invoke("cancel_download");
    log("Đang hủy... — .part được giữ lại để resume lần sau", "warn");
    $("#btn-cancel-download").disabled = true;
  } catch (err) {
    log(String(err), "error");
  }
}

function setupDownloadEvents() {
  if (!listen) return;
  listen("download://log", (e) => {
    const { message, level } = e.payload;
    log(message, LOG_LEVEL_MAP[level] ?? "info");
  });
  listen("download://progress", (e) => {
    const p = e.payload;
    $("#dl-overall-pct").textContent = `${(p.overallPct ?? 0).toFixed(1)}%`;
    $("#dl-bar-overall").style.width = `${Math.min(100, p.overallPct ?? 0)}%`;
    $("#dl-pkg-name").textContent = p.pkgName ?? "";
    $("#dl-pkg-pct").textContent = `${Math.round(p.pkgPct ?? 0)}%`;
    $("#dl-bar-pkg").style.width = `${Math.min(100, p.pkgPct ?? 0)}%`;
    $("#dl-speed").textContent = fmtSpeed(p.speedBps);
    $("#dl-eta").textContent = fmtEta(p.etaSecs);
  });
  listen("download://done", async (e) => {
    setDownloadBusy(false);
    const dir = e.payload?.installDir;
    if (dir) {
      refreshHistory();
      refreshInstalled(false);
      if (window.confirm(`Roblox Player đã sẵn sàng!\n\nThư mục:\n${dir}\n\nMở thư mục cài đặt?`)) {
        openFolder(dir);
      }
    }
  });
  listen("download://error", (e) => {
    const { title, message } = e.payload;
    setDownloadBusy(false);
    log(`Lỗi — ${title}: ${message}`, "error");
  });
}

function validateCustomHash() {
  const raw = $("#custom-hash").value.trim().toLowerCase().replace(/^version-/, "");
  const valid = /^[0-9a-f]{16}$/.test(raw);
  const preview = $("#hash-preview");
  if (!raw) {
    preview.textContent = "";
    preview.classList.remove("invalid");
    return null;
  }
  if (valid) {
    preview.textContent = `Sẽ tải: version-${raw} (manifest + toàn bộ .zip)`;
    preview.classList.remove("invalid");
    return `version-${raw}`;
  }
  preview.textContent = "Hash không hợp lệ — cần 16 ký tự hex sau dấu gạch";
  preview.classList.add("invalid");
  return null;
}

async function loadDeployHistory() {
  if (!invoke) return;
  const sel = $("#history-select");
  sel.innerHTML = "";
  try {
    const versions = await invoke("get_history_versions");
    for (const vh of versions) {
      const opt = document.createElement("option");
      opt.value = vh;
      opt.textContent = vh;
      sel.appendChild(opt);
    }
    log(`Nạp ${versions.length} version từ DeployHistory.txt`, "ok");
  } catch (err) {
    log(String(err), "error");
  }
}

function installedRow(v) {
  const row = document.createElement("div");
  row.className = "row";

  const hash = document.createElement("span");
  hash.className = "hash-value";
  hash.textContent = v.hash;

  const badges = document.createElement("span");
  badges.className = "badges";
  if (v.hasPlayer) badges.appendChild(badge("Player", "ok"));
  if (v.hasStudio) badges.appendChild(badge("Studio", "info"));

  const size = document.createElement("span");
  size.className = "muted";
  size.textContent = v.sizeBytes ? fmtBytes(v.sizeBytes) : "";

  const actions = document.createElement("span");
  actions.className = "actions";

  const play = btn("Chạy", () => runVersion(v.hash));
  const open = btn("Thư mục", () => openFolder(v.path));
  const del = btn("Xóa", () => deleteVersionFlow(del, v.hash));
  if (!v.hasPlayer) play.disabled = true;

  actions.append(play, open, del);
  row.append(hash, badges, size, actions);
  return row;
}

async function refreshInstalled(withSizes = true) {
  if (!invoke) return;
  const listEl = $("#installed-list");
  listEl.innerHTML = "";
  try {
    const list = await invoke("list_installed_versions", { withSizes });
    if (!list.length) {
      listEl.appendChild(emptyRow("Không tìm thấy version nào trong %LOCALAPPDATA%\\Roblox\\Versions"));
      return;
    }
    for (const v of list) {
      listEl.appendChild(installedRow(v));
    }
    log(`Tìm thấy ${list.length} version trên máy`, "ok");
  } catch (err) {
    log(String(err), "error");
  }
}

async function runVersion(h) {
  try {
    await invoke("launch_version", { hash: h });
    log(`Đã khởi chạy ${h}`, "ok");
  } catch (err) {
    log(String(err), "error");
  }
}

async function openFolder(p) {
  try {
    await invoke("open_folder", { path: p });
  } catch (err) {
    log(String(err), "error");
  }
}

function disarmDelete(delBtn) {
  state.armDelete = null;
  delBtn.textContent = "Xóa";
  delBtn.classList.remove("danger");
}

function deleteVersionFlow(delBtn, hash) {
  if (state.armDelete === hash) {
    doDeleteVersion(delBtn, hash);
    return;
  }
  state.armDelete = hash;
  delBtn.textContent = "Xóa thật?";
  delBtn.classList.add("danger");
  clearTimeout(state.armTimer);
  state.armTimer = setTimeout(() => disarmDelete(delBtn), 4000);
}

async function doDeleteVersion(delBtn, hash) {
  delBtn.disabled = true;
  try {
    await invoke("delete_version", { hash });
    log(`Đã xóa ${hash}`, "ok");
    refreshInstalled();
  } catch (err) {
    log(String(err), "error");
    delBtn.disabled = false;
  }
}

async function clearCacheFlow() {
  try {
    const removed = await invoke("clear_cache");
    log(`Đã dọn cache: giải phóng ${fmtBytes(removed)}`, "ok");
  } catch (err) {
    log(String(err), "error");
  }
}

async function refreshHistory() {
  if (!invoke) return;
  const listEl = $("#history-list");
  listEl.innerHTML = "";
  const entries = await invoke("get_download_history");
  if (!entries.length) {
    listEl.appendChild(emptyRow("Chưa có lịch sử"));
    return;
  }
  for (const e of entries) {
    const row = document.createElement("div");
    row.className = "row";
    const hash = document.createElement("span");
    hash.className = "hash-value";
    hash.textContent = e.hash;
    const time = document.createElement("span");
    time.className = "muted";
    time.textContent = e.time ?? "";
    const actions = document.createElement("span");
    actions.className = "actions";
    actions.appendChild(btn("Điền hash", () => {
      $("#custom-hash").value = e.hash;
      validateCustomHash();
    }));
    if (e.path) {
      actions.appendChild(btn("Thư mục", () => openFolder(e.path)));
    }
    row.append(hash, time, actions);
    listEl.appendChild(row);
  }
}

async function importLegacyHistory() {
  const suggestion = "C:\\Users\\Long\\Desktop\\PY\\RobloxVersionDownloader\\backup\\download_history.json";
  const p = window.prompt("Đường dẫn đến download_history.json cũ:", suggestion);
  if (!p) return;
  try {
    const saved = await invoke("import_legacy_history", { filePath: p });
    log(`Đã import lịch sử (${saved} bản ghi)`, "ok");
    refreshHistory();
  } catch (err) {
    log(String(err), "error");
  }
}

document.addEventListener("DOMContentLoaded", () => {
  $("#btn-refresh-live").addEventListener("click", refreshLive);
  $("#btn-copy-latest").addEventListener("click", copyLatest);
  $("#btn-download-latest").addEventListener("click", () => {
    const h = state.live?.latest;
    if (h) downloadHash(h);
    else log("Chưa có hash mới nhất — bấm Làm mới", "error");
  });
  $("#btn-refresh-installed").addEventListener("click", () => refreshInstalled(true));
  $("#btn-clear-cache").addEventListener("click", clearCacheFlow);
  $("#btn-load-history").addEventListener("click", loadDeployHistory);
  $("#history-select").addEventListener("change", (e) => {
    const v = e.target.value;
    if (!v) return;
    $("#custom-hash").value = v;
    validateCustomHash();
  });
  $("#custom-hash").addEventListener("input", validateCustomHash);
  $("#btn-download-custom").addEventListener("click", () => {
    const h = validateCustomHash();
    if (h) downloadHash(h);
    else log("Hash tùy chọn không hợp lệ", "error");
  });
  $("#btn-import-history").addEventListener("click", importLegacyHistory);
  $("#btn-cancel-download").addEventListener("click", cancelDownload);

  if (!invoke) {
    log("window.__TAURI__ không có — hãy chạy qua `npm run dev` (Tauri)", "error");
    return;
  }
  setDownloadBusy(false);
  setupDownloadEvents();
  refreshLive();
  refreshInstalled(true);
  refreshHistory();
});
