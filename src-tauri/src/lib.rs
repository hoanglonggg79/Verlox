mod cdn;
mod extract;
mod hash;
mod history;
mod installed;
mod manifest;
mod versions;
mod win;

use installed::{
    clear_dir_contents, downloads_root, is_unsafe_dir, versions_root, InstalledVersion,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("Verlox/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("khởi tạo HTTP client thất bại")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LiveVersions {
    latest: Option<String>,
    studio_latest: Option<String>,
    future_windows: Option<String>,
    previous_windows: Option<String>,
    previous_mac: Option<String>,
}

#[tauri::command]
async fn get_live_versions() -> Result<LiveVersions, String> {
    let client = http_client();
    let (previous_windows, previous_mac) = versions::fetch_previous_pair(&client).await;
    Ok(LiveVersions {
        latest: versions::fetch_latest(&client).await,
        studio_latest: versions::fetch_studio_latest(&client).await,
        future_windows: versions::fetch_future_windows(&client).await,
        previous_windows,
        previous_mac,
    })
}

#[tauri::command]
async fn get_history_versions() -> Result<Vec<String>, String> {
    let client = http_client();
    let list = versions::fetch_deploy_history(&client).await;
    if list.is_empty() {
        return Err("Không lấy được danh sách version từ DeployHistory.txt".to_string());
    }
    Ok(list)
}

#[tauri::command]
async fn list_executors() -> Result<Vec<versions::ExecutorInfo>, String> {
    let client = http_client();
    versions::fetch_executors(&client).await
}

#[tauri::command]
fn list_installed_versions(with_sizes: Option<bool>) -> Result<Vec<InstalledVersion>, String> {
    installed::scan(with_sizes.unwrap_or(false))
}

#[tauri::command]
fn get_download_history() -> Vec<history::HistoryEntry> {
    history::load()
}

#[tauri::command]
fn import_legacy_history(file_path: String) -> Result<usize, String> {
    let text = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Không đọc được {}: {e}", file_path))?;
    let parsed: Vec<history::HistoryEntry> =
        serde_json::from_str(&text).map_err(|e| format!("JSON cũ không hợp lệ: {e}"))?;
    let mut entries = parsed;
    for e in history::load() {
        if !entries.iter().any(|x| x.hash == e.hash) {
            entries.push(e);
        }
    }
    entries.truncate(history::MAX_HISTORY);
    let saved = entries.len();
    history::save(&entries)?;
    Ok(saved)
}

#[tauri::command]
fn launch_version(hash: String) -> Result<(), String> {
    let vh = hash::validate_hash(&hash).ok_or_else(|| "Hash không hợp lệ".to_string())?;
    let dir = versions_root().join(&vh);
    let exe = dir.join("RobloxPlayerBeta.exe");
    if !exe.is_file() {
        return Err(format!("Không tìm thấy {}", exe.display()));
    }
    std::process::Command::new(&exe)
        .current_dir(&dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.is_dir() {
        return Err(format!("Thư mục không tồn tại: {path}"));
    }
    std::process::Command::new("explorer.exe")
        .arg(&p)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_version(hash: String) -> Result<(), String> {
    let vh = hash::validate_hash(&hash).ok_or_else(|| "Hash không hợp lệ".to_string())?;
    let dir = versions_root().join(&vh);
    if !dir.is_dir() {
        return Err(format!("Phiên bản không tồn tại: {vh}"));
    }
    std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_cache(state: tauri::State<'_, DownloadState>) -> Result<u64, String> {
    if state.running.load(Ordering::SeqCst) {
        return Err(
            "Đang có tác vụ tải đang chạy — không thể dọn cache lúc này, thử lại sau khi tải xong."
                .to_string(),
        );
    }
    clear_dir_contents(&downloads_root())
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct DownloadOptions {
    #[serde(default)]
    register_protocol: bool,
    #[serde(default)]
    install_webview2: bool,
    #[serde(default)]
    create_shortcut: bool,
    #[serde(default)]
    overwrite: bool,
}

#[derive(Default)]
pub struct DownloadState {
    cancel: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
}

struct RunningGuard(Arc<AtomicBool>);

impl Drop for RunningGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn dir_non_empty(p: &Path) -> bool {
    std::fs::read_dir(p)
        .map(|mut rd| rd.next().is_some())
        .unwrap_or(false)
}

fn emit_log(app: &tauri::AppHandle, message: &str, level: &str) {
    let _ = app.emit(
        "download://log",
        json!({ "message": message, "level": level }),
    );
}

fn emit_progress_values(
    app: &tauri::AppHandle,
    overall_pct: f64,
    pkg_name: &str,
    pkg_pct: f64,
    speed_bps: f64,
    eta_secs: f64,
) {
    let _ = app.emit(
        "download://progress",
        json!({
            "overallPct": overall_pct,
            "pkgName": pkg_name,
            "pkgPct": pkg_pct,
            "speedBps": speed_bps,
            "etaSecs": eta_secs,
        }),
    );
}

fn ps_escape(s: &str) -> String {
    s.replace('\'', "''")
}

fn make_shortcut(exe: &Path, vh: &str) -> Result<(), String> {
    let base = std::env::var("USERPROFILE").map_err(|e| e.to_string())?;
    let lnk = PathBuf::from(base)
        .join("Desktop")
        .join(format!("Roblox {vh}.lnk"));
    let dir = exe.parent().unwrap_or(Path::new("."));
    let ps = format!(
        "$s=(New-Object -COM WScript.Shell).CreateShortcut('{lnk}');$s.TargetPath='{exe}';$s.WorkingDirectory='{dir}';$s.Description='Roblox Player {vh}';$s.Save()",
        lnk = ps_escape(&lnk.to_string_lossy()),
        exe = ps_escape(&exe.to_string_lossy()),
        dir = ps_escape(&dir.to_string_lossy()),
        vh = vh
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps.as_str()])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    Ok(())
}

#[tauri::command]
async fn start_download(
    app: tauri::AppHandle,
    state: tauri::State<'_, DownloadState>,
    hash: String,
    install_dir: Option<String>,
    options: Option<DownloadOptions>,
) -> Result<(), String> {
    let vh = hash::validate_hash(&hash).ok_or_else(|| {
        "Hash không hợp lệ: cần version-xxxxxxxxxxxxxxxx (16 ký tự hex)".to_string()
    })?;
    let dir = match install_dir.as_deref() {
        Some(d) if !d.trim().is_empty() => PathBuf::from(d).join(&vh),
        _ => versions_root().join(&vh),
    };
    if is_unsafe_dir(&dir) {
        return Err("Thư mục cài đặt nằm trong khu vực hệ thống không an toàn".to_string());
    }
    let opts = options.unwrap_or_default();
    let client = cdn::download_client()?;
    if state.running.swap(true, Ordering::SeqCst) {
        return Err("Đang có một tác vụ tải khác chạy — hãy hủy hoặc đợi nó xong.".to_string());
    }
    if dir.is_dir() && dir_non_empty(&dir) && !opts.overwrite {
        state.running.store(false, Ordering::SeqCst);
        return Err(format!("EXISTS::{}", dir.display()));
    }
    state.cancel.store(false, Ordering::SeqCst);
    let cancel = state.cancel.clone();
    let running = state.running.clone();
    tauri::async_runtime::spawn(async move {
        let _guard = RunningGuard(running);
        install_task(app, client, vh, dir, opts, cancel).await;
    });
    Ok(())
}

#[tauri::command]
fn cancel_download(state: tauri::State<'_, DownloadState>) -> Result<(), String> {
    state.cancel.store(true, Ordering::SeqCst);
    Ok(())
}

async fn install_task(
    app: tauri::AppHandle,
    client: reqwest::Client,
    vh: String,
    install_dir: PathBuf,
    opts: DownloadOptions,
    cancel: Arc<AtomicBool>,
) {
    let result = install_inner(&app, &client, &vh, &install_dir, &opts, &cancel).await;
    match result {
        Ok(true) => {
            emit_log(
                &app,
                &format!("Hoàn tất! → {}", install_dir.display()),
                "success",
            );
            let _ = app.emit(
                "download://done",
                json!({ "installDir": install_dir.to_string_lossy() }),
            );
        }
        Ok(false) => {
            let _ = app.emit("download://done", json!({ "installDir": null }));
        }
        Err((title, message)) => {
            emit_log(&app, &title, "error");
            let _ = app.emit(
                "download://error",
                json!({ "title": title, "message": message }),
            );
        }
    }
}

async fn install_inner(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    vh: &str,
    install_dir: &Path,
    opts: &DownloadOptions,
    cancel: &AtomicBool,
) -> Result<bool, (String, String)> {
    emit_log(app, "Đang kiểm tra CDN...", "warning");
    let cdn = cdn::probe_cdns(client, vh).await;
    emit_log(app, &format!("Dùng CDN: {cdn}"), "muted");
    if cancel.load(Ordering::Relaxed) {
        emit_log(app, "Đã hủy.", "muted");
        return Ok(false);
    }

    emit_log(app, "Đang tải danh sách gói (manifest)...", "warning");
    let text = cdn::fetch_manifest_text(client, cdn, vh)
        .await
        .map_err(|e| ("Không lấy được manifest".to_string(), e))?;
    if cancel.load(Ordering::Relaxed) {
        emit_log(app, "Đã hủy.", "muted");
        return Ok(false);
    }

    let packages = manifest::parse_manifest(&text);
    if packages.is_empty() {
        return Err((
            "Manifest trống".to_string(),
            "Không đọc được danh sách gói từ CDN.\nThử lại hoặc kiểm tra version hash.".to_string(),
        ));
    }
    let zips: Vec<&manifest::Package> = packages
        .iter()
        .filter(|p| p.name.ends_with(".zip"))
        .collect();
    let total_bytes: u64 = zips.iter().map(|p| p.compressed).sum();
    emit_log(
        app,
            &format!(
                "Tìm thấy {} gói ({})",
                zips.len(),
                cdn::fmt_bytes(total_bytes)
            ),
        "muted",
    );

    std::fs::create_dir_all(install_dir).map_err(|e| {
        (
            "Không tạo được thư mục cài đặt".to_string(),
            format!("{}\n\n{e}", install_dir.display()),
        )
    })?;
    let cache_dir = downloads_root();
    std::fs::create_dir_all(&cache_dir).map_err(|e| {
        (
            "Không tạo được thư mục cache".to_string(),
            format!("{}\n\n{e}", cache_dir.display()),
        )
    })?;

    let mut downloaded: u64 = 0;
    for (idx, pkg) in zips.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            emit_log(app, "Đã hủy.", "muted");
            return Ok(false);
        }
        let name = &pkg.name;
        let cache_path = cache_dir.join(format!("{vh}-{name}"));
        emit_log(
            app,
            &format!("[{}/{}]  {}", idx + 1, zips.len(), name),
            "warning",
        );

        if cache_path.is_file() {
            let ok = cdn::md5_file(&cache_path)
                .await
                .map(|m| m.eq_ignore_ascii_case(&pkg.md5))
                .unwrap_or(false);
            if ok {
                emit_log(app, "Cache OK, bỏ qua tải.", "muted");
            } else {
                let _ = std::fs::remove_file(&cache_path);
                emit_log(app, "Cache hỏng, tải lại.", "warning");
            }
        }

        if !cache_path.is_file() {
            let n = name.clone();
            let app2 = app.clone();
            let finished = cdn::download_pkg(
                client,
                cdn,
                vh,
                pkg,
                &cache_path,
                downloaded,
                total_bytes,
                cancel,
                |pr| {
                    emit_progress_values(
                        &app2,
                        pr.overall_pct,
                        &n,
                        pr.pkg_pct,
                        pr.speed_bps,
                        pr.eta_secs,
                    )
                },
            )
            .await
            .map_err(|e| ("Lỗi tải gói".to_string(), e))?;
            if !finished {
                emit_log(app, "Đã hủy.", "muted");
                return Ok(false);
            }
        }

        downloaded += pkg.compressed;
        let overall_pct = if total_bytes > 0 {
            downloaded as f64 / total_bytes as f64 * 100.0
        } else {
            0.0
        };
        emit_progress_values(app, overall_pct, name, 100.0, 0.0, 0.0);

        if cancel.load(Ordering::Relaxed) {
            emit_log(app, "Đã hủy.", "muted");
            return Ok(false);
        }
        emit_log(app, &format!("Giải nén {name}..."), "muted");
        let zip_path = cache_path.clone();
        let out_dir = install_dir.to_path_buf();
        let pkg_name = name.clone();
        let w2 = opts.install_webview2;
        tokio::task::spawn_blocking(move || {
            extract::extract_pkg(&zip_path, &out_dir, &pkg_name, w2)
        })
        .await
        .map_err(|e| ("Lỗi giải nén".to_string(), format!("{name}\n\n{e}")))?
        .map_err(|e| ("Lỗi giải nén".to_string(), format!("{name}\n\n{e}")))?;
    }

    if cancel.load(Ordering::Relaxed) {
        emit_log(app, "Đã hủy.", "muted");
        return Ok(false);
    }
    extract::write_app_settings(install_dir)
        .map_err(|e| ("Lỗi ghi AppSettings.xml".to_string(), e))?;
    emit_log(app, "Đã ghi AppSettings.xml.", "muted");

    if opts.register_protocol {
        let exe = install_dir.join("RobloxPlayerBeta.exe");
        if exe.is_file() {
            match win::register_protocol(&exe) {
                Ok(()) => emit_log(
                    app,
                    "Đã đăng ký protocol roblox-player:// cho phiên bản này.",
                    "success",
                ),
                Err(e) => emit_log(app, &format!("Không đăng ký được protocol: {e}"), "warning"),
            }
        } else {
            emit_log(
                app,
                "Không tìm thấy RobloxPlayerBeta.exe — bỏ qua đăng ký protocol.",
                "warning",
            );
        }
    }

    if opts.create_shortcut {
        let exe = install_dir.join("RobloxPlayerBeta.exe");
        if exe.is_file() {
            match make_shortcut(&exe, vh) {
                Ok(()) => emit_log(app, "Đã tạo shortcut trên Desktop.", "success"),
                Err(e) => emit_log(app, &format!("Không tạo được shortcut: {e}"), "warning"),
            }
        }
    }

    if let Err(e) = history::push(vh, &install_dir.to_string_lossy()) {
        emit_log(app, &format!("Không lưu được lịch sử: {e}"), "warning");
    }
    Ok(true)
}

#[tauri::command]
fn register_protocol(hash: String) -> Result<(), String> {
    let vh = hash::validate_hash(&hash).ok_or_else(|| "Hash không hợp lệ".to_string())?;
    let exe = versions_root().join(&vh).join("RobloxPlayerBeta.exe");
    if !exe.is_file() {
        return Err(format!("Không tìm thấy {}", exe.display()));
    }
    win::register_protocol(&exe)
}

#[tauri::command]
fn unregister_protocol() -> Result<(), String> {
    win::unregister_protocol()
}

#[tauri::command]
fn create_shortcut(hash: String) -> Result<(), String> {
    let vh = hash::validate_hash(&hash).ok_or_else(|| "Hash không hợp lệ".to_string())?;
    let exe = versions_root().join(&vh).join("RobloxPlayerBeta.exe");
    if !exe.is_file() {
        return Err(format!("Không tìm thấy {}", exe.display()));
    }
    make_shortcut(&exe, &vh)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(DownloadState::default())
        .invoke_handler(tauri::generate_handler![
            get_live_versions,
            get_history_versions,
            list_executors,
            list_installed_versions,
            get_download_history,
            import_legacy_history,
            launch_version,
            open_folder,
            delete_version,
            clear_cache,
            start_download,
            cancel_download,
            register_protocol,
            unregister_protocol,
            create_shortcut
        ])
        .run(tauri::generate_context!())
        .expect("Verlox gặp lỗi khi khởi chạy");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_guard_resets_on_drop() {
        let flag = Arc::new(AtomicBool::new(true));
        {
            let _guard = RunningGuard(flag.clone());
            assert!(flag.load(Ordering::SeqCst));
        }
        assert!(!flag.load(Ordering::SeqCst));
    }

    #[test]
    fn running_guard_resets_on_panic() {
        let flag = Arc::new(AtomicBool::new(true));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = RunningGuard(flag.clone());
            panic!("download task panic");
        }));
        assert!(result.is_err());
        assert!(!flag.load(Ordering::SeqCst));
    }
}
