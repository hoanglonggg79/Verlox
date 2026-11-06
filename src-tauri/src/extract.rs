use crate::manifest::{package_target, APP_SETTINGS_XML};
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn sanitize_entry_name(raw: &str) -> Option<PathBuf> {
    if raw.contains('\0') {
        return None;
    }
    let name = raw.replace('\\', "/");
    let name = match name.as_bytes() {
        [c, b':', rest @ ..] if c.is_ascii_alphabetic() => {
            String::from_utf8_lossy(rest).into_owned()
        }
        _ => name,
    };
    let name = name.trim_start_matches('/');
    let parts: Vec<&str> = name
        .split('/')
        .filter(|p| !p.is_empty() && *p != "." && *p != "..")
        .collect();
    if parts.is_empty() {
        return None;
    }
    Some(PathBuf::from(parts.join("/")))
}

fn extract_zip(zip_path: &Path, out_dir: &Path) -> Result<(), String> {
    let file =
        File::open(zip_path).map_err(|e| format!("Không mở được {}: {e}", zip_path.display()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("Zip lỗi: {e}"))?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("Zip lỗi: {e}"))?;
        let raw = entry.name().to_string();
        let Some(rel) = sanitize_entry_name(&raw) else {
            continue;
        };
        let dest = out_dir.join(rel);
        if raw.ends_with('/') || raw.ends_with('\\') {
            std::fs::create_dir_all(&dest)
                .map_err(|e| format!("Không tạo được {}: {e}", dest.display()))?;
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Không tạo được {}: {e}", parent.display()))?;
        }
        let out = File::create(&dest)
            .map_err(|e| format!("Không ghi được {}: {e}", dest.display()))?;
        let mut out = BufWriter::new(out);
        std::io::copy(&mut entry, &mut out).map_err(|e| format!("Giải nén lỗi: {e}"))?;
        out.flush().map_err(|e| format!("Không ghi được {}: {e}", dest.display()))?;
    }
    Ok(())
}

pub fn extract_pkg(
    zip_path: &Path,
    install_dir: &Path,
    pkg_name: &str,
    install_webview2: bool,
) -> Result<(), String> {
    let sub = package_target(pkg_name).unwrap_or("");
    if sub == "__webview2_installer__" {
        if !install_webview2 {
            return Ok(());
        }
        return run_webview2_installer(zip_path, install_dir);
    }
    let out_dir = if sub.is_empty() {
        install_dir.to_path_buf()
    } else {
        install_dir.join(sub)
    };
    std::fs::create_dir_all(&out_dir)
        .map_err(|e| format!("Không tạo được {}: {e}", out_dir.display()))?;
    extract_zip(zip_path, &out_dir)
}

fn run_webview2_installer(zip_path: &Path, install_dir: &Path) -> Result<(), String> {
    let installer_dir = install_dir.join("_webview2");
    std::fs::create_dir_all(&installer_dir).map_err(|e| e.to_string())?;
    let file =
        File::open(zip_path).map_err(|e| format!("Không mở được {}: {e}", zip_path.display()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("Zip lỗi: {e}"))?;

    let names: Vec<String> = zip.file_names().map(|s| s.to_string()).collect();
    let Some(idx) = names
        .iter()
        .position(|n| n.to_lowercase().ends_with(".exe"))
    else {
        return Ok(());
    };
    let mut data = Vec::new();
    zip.by_index(idx)
        .map_err(|e| format!("Zip lỗi: {e}"))?
        .read_to_end(&mut data)
        .map_err(|e| format!("Đọc zip lỗi: {e}"))?;
    let file_name = Path::new(&names[idx])
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let inst_exe = installer_dir.join(file_name);
    std::fs::write(&inst_exe, &data).map_err(|e| e.to_string())?;

    let Ok(mut child) = std::process::Command::new(&inst_exe)
        .args(["/silent", "/install"])
        .spawn()
    else {
        return Ok(());
    };
    for _ in 0..120u32 {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => std::thread::sleep(Duration::from_secs(1)),
            Err(_) => return Ok(()),
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

pub fn write_app_settings(install_dir: &Path) -> Result<(), String> {
    std::fs::write(install_dir.join("AppSettings.xml"), APP_SETTINGS_XML)
        .map_err(|e| format!("Không ghi được AppSettings.xml: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_zip(dir: &Path, name: &str, inner: &str) -> std::path::PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        let file = File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file(
            inner,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
        zip.write_all(b"hello").unwrap();
        zip.finish().unwrap();
        path
    }

    #[test]
    fn extracts_root_package_to_install_root() {
        let base = std::env::temp_dir().join("verlox-extract-test-root");
        let src = base.join("src");
        let dst = base.join("dst");
        let _ = std::fs::remove_dir_all(&dst);
        let zip_path = make_zip(&src, "RobloxApp.zip", "RobloxPlayerBeta.exe");
        extract_pkg(&zip_path, &dst, "RobloxApp.zip", false).unwrap();
        assert!(dst.join("RobloxPlayerBeta.exe").is_file());
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn extracts_content_package_into_subdir() {
        let base = std::env::temp_dir().join("verlox-extract-test-sub");
        let src = base.join("src");
        let dst = base.join("dst");
        let _ = std::fs::remove_dir_all(&dst);
        let zip_path = make_zip(&src, "content-avatar.zip", "avatar.fst");
        extract_pkg(&zip_path, &dst, "content-avatar.zip", false).unwrap();
        assert!(dst
            .join("content")
            .join("avatar")
            .join("avatar.fst")
            .is_file());
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn unknown_package_goes_to_root() {
        let base = std::env::temp_dir().join("verlox-extract-test-unknown");
        let src = base.join("src");
        let dst = base.join("dst");
        let _ = std::fs::remove_dir_all(&dst);
        let zip_path = make_zip(&src, "something-new.zip", "file.txt");
        extract_pkg(&zip_path, &dst, "something-new.zip", false).unwrap();
        assert!(dst.join("file.txt").is_file());
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn sanitizes_entry_names_like_python_extractall() {
        assert_eq!(sanitize_entry_name("\\"), None);
        assert_eq!(sanitize_entry_name("abilities\\"), Some(PathBuf::from("abilities")));
        assert_eq!(
            sanitize_entry_name("abilities\\AbilityManagerActor.rbxm"),
            Some(PathBuf::from("abilities/AbilityManagerActor.rbxm"))
        );
        assert_eq!(sanitize_entry_name("/abs/file.txt"), Some(PathBuf::from("abs/file.txt")));
        assert_eq!(sanitize_entry_name("../evil.txt"), Some(PathBuf::from("evil.txt")));
        assert_eq!(sanitize_entry_name("a/../../b.txt"), Some(PathBuf::from("a/b.txt")));
        assert_eq!(sanitize_entry_name("C:\\x\\y.txt"), Some(PathBuf::from("x/y.txt")));
        assert_eq!(sanitize_entry_name("bad\0name"), None);
    }

    #[test]
    fn extracts_roblox_style_backslash_zip() {
        let base = std::env::temp_dir().join("verlox-extract-test-backslash");
        let src = base.join("src");
        let dst = base.join("dst");
        let _ = std::fs::remove_dir_all(&dst);
        std::fs::create_dir_all(&src).unwrap();
        let zip_path = src.join("content-avatar.zip");
        let file = File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("\\", opts).unwrap();
        zip.write_all(b"").unwrap();
        zip.start_file("abilities\\", opts).unwrap();
        zip.write_all(b"").unwrap();
        zip.start_file("abilities\\AbilityManagerActor.rbxm", opts).unwrap();
        zip.write_all(b"hello").unwrap();
        zip.start_file("character.rbxm", opts).unwrap();
        zip.write_all(b"data").unwrap();
        zip.finish().unwrap();
        extract_pkg(&zip_path, &dst, "content-avatar.zip", false).unwrap();
        let avatar = dst.join("content").join("avatar");
        assert!(avatar.join("abilities").join("AbilityManagerActor.rbxm").is_file());
        assert_eq!(
            std::fs::read(avatar.join("abilities").join("AbilityManagerActor.rbxm")).unwrap(),
            b"hello"
        );
        assert!(avatar.join("character.rbxm").is_file());
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn webview2_skipped_when_option_off() {
        let base = std::env::temp_dir().join("verlox-extract-test-w2v");
        let src = base.join("src");
        let dst = base.join("dst");
        let _ = std::fs::remove_dir_all(&dst);
        let zip_path = make_zip(&src, "WebView2RuntimeInstaller.zip", "setup.exe");
        extract_pkg(&zip_path, &dst, "WebView2RuntimeInstaller.zip", false).unwrap();
        assert!(!dst.join("_webview2").exists());
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn writes_app_settings_xml() {
        let dir = std::env::temp_dir().join("verlox-extract-test-xml");
        std::fs::create_dir_all(&dir).unwrap();
        write_app_settings(&dir).unwrap();
        let text = std::fs::read_to_string(dir.join("AppSettings.xml")).unwrap();
        assert!(text.contains("<ContentFolder>content</ContentFolder>"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
