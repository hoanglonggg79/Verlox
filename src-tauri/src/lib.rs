mod cdn;
mod extract;
mod hash;
mod installed;
mod manifest;
mod versions;
mod win;

use installed::{
    clear_dir_contents, downloads_root, is_unsafe_dir, versions_root, InstalledVersion,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::Emitter;

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("Verlox/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("khởi tạo HTTP client thất bại")
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LiveVersions {
    latest: Option<String>,
    studio_latest: Option<String>,
    future_windows: Option<String>,
    previous_windows: Option<String>,
    previous_mac: Option<String>,
}

const API_CACHE_TTL_SECS: u64 = 300;

struct CacheEntry<T> {
    at: Instant,
    value: T,
}

#[derive(Default)]
struct ApiCache {
    live: Mutex<Option<CacheEntry<LiveVersions>>>,
    executors: Mutex<Option<CacheEntry<Vec<versions::ExecutorInfo>>>>,
    umi: Mutex<Option<CacheEntry<versions::ExecutorInfo>>>,
}

async fn fetch_cached<T: Clone>(
    slot: &Mutex<Option<CacheEntry<T>>>,
    force: bool,
    fetch: impl Future<Output = Result<T, String>>,
) -> Result<T, String> {
    let ttl = Duration::from_secs(API_CACHE_TTL_SECS);
    if !force {
        let fresh = slot.lock().ok().and_then(|g| {
            g.as_ref()
                .filter(|e| e.at.elapsed() < ttl)
                .map(|e| e.value.clone())
        });
        if let Some(value) = fresh {
            return Ok(value);
        }
    }
    match fetch.await {
        Ok(value) => {
            if let Ok(mut g) = slot.lock() {
                *g = Some(CacheEntry {
                    at: Instant::now(),
                    value: value.clone(),
                });
            }
            Ok(value)
        }
        Err(err) => {
            if !force {
                if let Ok(g) = slot.lock() {
                    if let Some(entry) = g.as_ref() {
                        return Ok(entry.value.clone());
                    }
                }
            }
            Err(err)
        }
    }
}

#[tauri::command]
async fn get_live_versions(
    cache: tauri::State<'_, ApiCache>,
    force: Option<bool>,
) -> Result<LiveVersions, String> {
    let force = force.unwrap_or(false);
    let client = http_client();
    fetch_cached(&cache.live, force, async {
        let (previous_windows, previous_mac) = versions::fetch_previous_pair(&client).await;
        Ok(LiveVersions {
            latest: versions::fetch_latest(&client).await,
            studio_latest: versions::fetch_studio_latest(&client).await,
            future_windows: versions::fetch_future_windows(&client).await,
            previous_windows,
            previous_mac,
        })
    })
    .await
}

#[tauri::command]
async fn list_executors(
    cache: tauri::State<'_, ApiCache>,
    force: Option<bool>,
) -> Result<Vec<versions::ExecutorInfo>, String> {
    let client = http_client();
    fetch_cached(&cache.executors, force.unwrap_or(false), async {
        versions::fetch_executors(&client).await
    })
    .await
}

#[tauri::command]
async fn get_umi_executor(
    cache: tauri::State<'_, ApiCache>,
    force: Option<bool>,
) -> Result<versions::ExecutorInfo, String> {
    let client = http_client();
    fetch_cached(&cache.umi, force.unwrap_or(false), async {
        versions::fetch_umi_executor(&client).await
    })
    .await
}

#[tauri::command]
fn list_installed_versions(with_sizes: Option<bool>) -> Result<Vec<InstalledVersion>, String> {
    installed::scan(with_sizes.unwrap_or(false))
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
fn delete_version(hash: String, state: tauri::State<'_, DownloadState>) -> Result<(), String> {
    let vh = hash::validate_hash(&hash).ok_or_else(|| "Hash không hợp lệ".to_string())?;
    let dir = versions_root().join(&vh);
    if !dir.is_dir() {
        return Err(format!("Phiên bản không tồn tại: {vh}"));
    }
    if let Ok(slot) = state.current_dir.lock() {
        if is_active_download_target(slot.as_deref(), &dir) {
            return Err(
                "Không xóa được — version này đang được tải dở.\nHủy tải hoặc đợi hoàn tất rồi xóa lại."
                    .to_string(),
            );
        }
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
    current_dir: Arc<Mutex<Option<PathBuf>>>,
}

struct RunningGuard {
    running: Arc<AtomicBool>,
    current_dir: Arc<Mutex<Option<PathBuf>>>,
}

impl Drop for RunningGuard {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Ok(mut slot) = self.current_dir.lock() {
            *slot = None;
        }
    }
}

fn is_active_download_target(current: Option<&Path>, dir: &Path) -> bool {
    current == Some(dir)
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
    if let Ok(mut slot) = state.current_dir.lock() {
        *slot = Some(dir.clone());
    }
    let cancel = state.cancel.clone();
    let running = state.running.clone();
    let current_dir = state.current_dir.clone();
    tauri::async_runtime::spawn(async move {
        let _guard = RunningGuard {
            running,
            current_dir,
        };
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
        .manage(ApiCache::default())
        .invoke_handler(tauri::generate_handler![
            get_live_versions,
            list_executors,
            get_umi_executor,
            list_installed_versions,
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

    fn guard_fixture() -> (Arc<AtomicBool>, Arc<Mutex<Option<PathBuf>>>) {
        (
            Arc::new(AtomicBool::new(true)),
            Arc::new(Mutex::new(Some(PathBuf::from(r"C:\roblox\version-abc")))),
        )
    }

    #[test]
    fn running_guard_resets_on_drop() {
        let (flag, dir) = guard_fixture();
        {
            let _guard = RunningGuard {
                running: flag.clone(),
                current_dir: dir.clone(),
            };
            assert!(flag.load(Ordering::SeqCst));
            assert_eq!(
                *dir.lock().unwrap(),
                Some(PathBuf::from(r"C:\roblox\version-abc"))
            );
        }
        assert!(!flag.load(Ordering::SeqCst));
        assert_eq!(*dir.lock().unwrap(), None);
    }

    #[test]
    fn running_guard_resets_on_panic() {
        let (flag, dir) = guard_fixture();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = RunningGuard {
                running: flag.clone(),
                current_dir: dir.clone(),
            };
            panic!("download task panic");
        }));
        assert!(result.is_err());
        assert!(!flag.load(Ordering::SeqCst));
        assert_eq!(*dir.lock().unwrap(), None);
    }

    #[test]
    fn active_download_target_matches_only_exact_dir() {
        let target = Path::new(r"C:\roblox\Versions\version-abc");
        assert!(is_active_download_target(Some(target), target));
        assert!(!is_active_download_target(
            Some(Path::new(r"C:\roblox\Versions\version-xyz")),
            target
        ));
        assert!(!is_active_download_target(None, target));
    }

    #[tokio::test]
    async fn cache_serves_fresh_hit_without_fetch() {
        let slot: Mutex<Option<CacheEntry<u32>>> = Mutex::new(Some(CacheEntry {
            at: Instant::now(),
            value: 7,
        }));
        let out = fetch_cached(&slot, false, async { Err::<u32, _>("boom".to_string()) }).await;
        assert_eq!(out.unwrap(), 7);
    }

    #[tokio::test]
    async fn cache_serves_stale_on_fetch_error() {
        let stale_at = Instant::now()
            .checked_sub(Duration::from_secs(600))
            .unwrap();
        let slot: Mutex<Option<CacheEntry<u32>>> = Mutex::new(Some(CacheEntry {
            at: stale_at,
            value: 3,
        }));
        let out = fetch_cached(&slot, false, async { Err::<u32, _>("429".to_string()) }).await;
        assert_eq!(out.unwrap(), 3);
    }

    #[tokio::test]
    async fn cache_force_bypasses_hit_and_error_propagates_when_empty() {
        let slot: Mutex<Option<CacheEntry<u32>>> = Mutex::new(Some(CacheEntry {
            at: Instant::now(),
            value: 7,
        }));
        let out = fetch_cached(&slot, true, async { Err::<u32, _>("boom".to_string()) }).await;
        assert!(out.is_err());

        let empty: Mutex<Option<CacheEntry<u32>>> = Mutex::new(None);
        let out = fetch_cached(&empty, false, async { Err::<u32, _>("boom".to_string()) }).await;
        assert!(out.is_err());

        let out = fetch_cached(&empty, false, async { Ok::<u32, _>(9) }).await;
        assert_eq!(out.unwrap(), 9);
        assert_eq!(empty.lock().unwrap().as_ref().map(|e| e.value), Some(9));
    }
}
