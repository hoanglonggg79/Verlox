use crate::manifest::Package;
use futures::StreamExt;
use md5::{Digest, Md5};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const CDN_BASE: &str = "https://setup.rbxcdn.com";
pub const CDN_FALLBACKS: &[&str] = &[
    "https://setup-ak.rbxcdn.com",
    "https://setup-cf.rbxcdn.com",
    "https://roblox-setup.cachefly.net",
    "https://s3.amazonaws.com/setup.roblox.com",
];

const PROBE_TIMEOUT_SECS: u64 = 5;
const PROGRESS_INTERVAL_SECS: f64 = 0.4;

pub fn download_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("Verlox/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(12))
        .read_timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Không khởi tạo được HTTP client: {e}"))
}

pub fn fmt_bytes(n: u64) -> String {
    if n < 1024 {
        return format!("{n} B");
    }
    let kb = n as f64 / 1024.0;
    if kb < 1024.0 {
        return format!("{kb:.1} KB");
    }
    let mb = kb / 1024.0;
    if mb < 1024.0 {
        return format!("{mb:.2} MB");
    }
    format!("{:.2} GB", mb / 1024.0)
}

pub async fn md5_file(path: &Path) -> Result<String, String> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| format!("Không đọc được {}: {e}", path.display()))?;
    let mut hasher = Md5::new();
    let mut buf = vec![0u8; 65536];
    loop {
        let n = file
            .read(&mut buf)
            .await
            .map_err(|e| format!("Không đọc được {}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn part_path(dest: &Path) -> PathBuf {
    let mut s = dest.as_os_str().to_os_string();
    s.push(".part");
    PathBuf::from(s)
}

fn network_err(name: &str, e: reqwest::Error) -> String {
    if e.is_timeout() {
        format!("Timeout khi tải:\n{name}")
    } else if e.is_connect() {
        format!(
            "Mất kết nối khi tải:\n{name}\n\nThử lại — sẽ resume từ chỗ dừng (file .part được giữ lại)."
        )
    } else {
        format!("Lỗi mạng khi tải {name}: {e}")
    }
}

pub async fn probe_cdns(client: &reqwest::Client, vh: &str) -> &'static str {
    let all: Vec<&'static str> = std::iter::once(CDN_BASE)
        .chain(CDN_FALLBACKS.iter().copied())
        .collect();
    let probes = all.iter().map(|cdn| {
        let url = format!("{cdn}/{vh}-rbxPkgManifest.txt");
        async move {
            let t0 = Instant::now();
            let ok = match client
                .head(&url)
                .timeout(Duration::from_secs(PROBE_TIMEOUT_SECS))
                .send()
                .await
            {
                Ok(r) => r.status() == reqwest::StatusCode::OK,
                Err(_) => false,
            };
            let dt = if ok {
                t0.elapsed().as_secs_f64()
            } else {
                f64::INFINITY
            };
            (*cdn, dt)
        }
    });
    let mut best = (CDN_BASE, f64::INFINITY);
    for (cdn, dt) in futures::future::join_all(probes).await {
        if dt < best.1 {
            best = (cdn, dt);
        }
    }
    best.0
}

pub async fn fetch_manifest_text(
    client: &reqwest::Client,
    cdn: &str,
    vh: &str,
) -> Result<String, String> {
    let url = format!("{cdn}/{vh}-rbxPkgManifest.txt");
    let resp = client.get(&url).send().await.map_err(|_| {
        "Không kết nối được CDN để lấy manifest — kiểm tra mạng rồi thử lại.".to_string()
    })?;
    match resp.status().as_u16() {
        404 => {
            return Err(format!(
                "Hash không tồn tại trên CDN:\n{vh}\n\nHãy kiểm tra lại version hash."
            ))
        }
        s if !(200..300).contains(&s) => {
            return Err(format!("CDN trả mã HTTP {s} khi lấy manifest"));
        }
        _ => {}
    }
    resp.text()
        .await
        .map_err(|e| format!("Không đọc được manifest: {e}"))
}

pub struct PkgProgress {
    pub overall_pct: f64,
    pub pkg_pct: f64,
    pub speed_bps: f64,
    pub eta_secs: f64,
}

pub async fn download_pkg<F>(
    client: &reqwest::Client,
    cdn: &str,
    vh: &str,
    pkg: &Package,
    dest: &Path,
    done_bytes: u64,
    total_bytes: u64,
    cancel: &AtomicBool,
    mut on_progress: F,
) -> Result<bool, String>
where
    F: FnMut(PkgProgress),
{
    let url = format!("{cdn}/{vh}-{}", pkg.name);
    let part = part_path(dest);

    let mut resume_pos = 0u64;
    let mut req = client.get(&url);
    if part.is_file() {
        resume_pos = std::fs::metadata(&part)
            .map_err(|e| format!("Không đọc được {}: {e}", part.display()))?
            .len();
        req = req.header(reqwest::header::RANGE, format!("bytes={resume_pos}-"));
    }

    let mut resp = req.send().await.map_err(|e| network_err(&pkg.name, e))?;

    if resp.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        let _ = std::fs::remove_file(&part);
        resume_pos = 0;
        resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| network_err(&pkg.name, e))?;
    }

    let partial = resp.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    if resume_pos > 0 && !partial {
        resume_pos = 0;
    }

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(format!("CDN không có gói:\n{}", pkg.name));
    }
    if !resp.status().is_success() {
        return Err(format!("Lỗi HTTP {} khi tải {}", resp.status(), pkg.name));
    }

    let pkg_size = resp.content_length().map_or(0, |len| len + resume_pos);
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(resume_pos > 0)
        .truncate(resume_pos == 0)
        .open(&part)
        .await
        .map_err(|e| {
            format!(
                "Không ghi được vào {} — đĩa đầy hoặc không đủ quyền:\n{e}",
                part.display()
            )
        })?;

    let mut pkg_done = resume_pos;
    let mut last_t = Instant::now();
    let mut last_b = done_bytes + resume_pos;

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            let _ = file.flush().await;
            return Ok(false);
        }
        let chunk = chunk.map_err(|e| network_err(&pkg.name, e))?;
        file.write_all(&chunk).await.map_err(|e| {
            format!(
                "Không ghi được {} — đĩa đầy hoặc không đủ quyền:\n{e}",
                pkg.name
            )
        })?;
        pkg_done += chunk.len() as u64;
        let now_bytes = done_bytes + pkg_done;
        let dt = last_t.elapsed().as_secs_f64();
        if dt >= PROGRESS_INTERVAL_SECS {
            let speed = (now_bytes - last_b) as f64 / dt;
            last_t = Instant::now();
            last_b = now_bytes;
            let eta = if speed > 0.0 && total_bytes > now_bytes {
                (total_bytes - now_bytes) as f64 / speed
            } else {
                0.0
            };
            on_progress(PkgProgress {
                overall_pct: if total_bytes > 0 {
                    now_bytes as f64 / total_bytes as f64 * 100.0
                } else {
                    0.0
                },
                pkg_pct: if pkg_size > 0 {
                    pkg_done as f64 / pkg_size as f64 * 100.0
                } else {
                    0.0
                },
                speed_bps: speed,
                eta_secs: eta,
            });
        }
    }
    file.flush()
        .await
        .map_err(|e| format!("Không ghi được {}: {e}", pkg.name))?;

    let actual = md5_file(&part).await?;
    if !actual.eq_ignore_ascii_case(&pkg.md5) {
        let _ = std::fs::remove_file(&part);
        return Err(format!(
            "Checksum thất bại — gói hỏng trong lúc tải:\n{}\n\nKỳ vọng: {}\nNhận được: {}\n\nLần sau sẽ tự động tải lại.",
            pkg.name, pkg.md5, actual
        ));
    }

    tokio::fs::rename(&part, dest)
        .await
        .map_err(|e| format!("Không hoàn tất {}: {e}", dest.display()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn md5_hex(data: &[u8]) -> String {
        let mut hasher = Md5::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    #[test]
    fn md5_known_vector() {
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn fmt_bytes_units() {
        assert_eq!(fmt_bytes(512), "512 B");
        assert_eq!(fmt_bytes(2048), "2.0 KB");
        assert_eq!(fmt_bytes(5 * 1024 * 1024), "5.00 MB");
        assert_eq!(fmt_bytes(3 * 1024 * 1024 * 1024), "3.00 GB");
    }

    #[test]
    fn part_extends_name() {
        let p = part_path(Path::new(r"C:\cache\version-abc-RobloxApp.zip"));
        assert_eq!(p, PathBuf::from(r"C:\cache\version-abc-RobloxApp.zip.part"));
    }

    #[tokio::test]
    async fn md5_file_matches_vector() {
        let dir = std::env::temp_dir().join("verlox-cdn-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("md5-sample.bin");
        std::fs::write(&p, b"abc").unwrap();
        assert_eq!(
            md5_file(&p).await.unwrap(),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        std::fs::remove_file(&p).unwrap();
    }
}
