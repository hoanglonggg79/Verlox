use crate::hash::validate_hash;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub fn roblox_local_root() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base)
}

pub fn versions_root() -> PathBuf {
    roblox_local_root().join("Roblox").join("Versions")
}

pub fn downloads_root() -> PathBuf {
    roblox_local_root().join("Roblox").join("Downloads")
}

pub fn verlox_root() -> PathBuf {
    roblox_local_root().join("Verlox")
}

const UNSAFE_EXACT: &[&str] = &[
    "c:\\windows",
    "c:\\windows\\system32",
    "c:\\program files",
    "c:\\program files (x86)",
    "c:\\",
    "/",
    "/usr",
    "/etc",
    "/bin",
    "/system",
    "/library",
];

const RISKY_PREFIX: &[&str] = &[
    "c:\\windows",
    "c:\\windows\\system32",
    "/usr",
    "/etc",
    "/bin",
    "/system",
    "/library",
];

fn normalized(p: &Path) -> String {
    let resolved = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let raw = resolved.to_string_lossy().to_lowercase();
    raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_string()
}

pub fn is_unsafe_dir(path: &Path) -> bool {
    let s = normalized(path);
    let trimmed = s.trim_end_matches(['\\', '/']);
    for u in UNSAFE_EXACT {
        if trimmed == u.trim_end_matches(['\\', '/']) {
            return true;
        }
    }
    for u in RISKY_PREFIX {
        let ut = u.trim_end_matches(['\\', '/']);
        if s.starts_with(&format!("{ut}\\")) || s.starts_with(&format!("{ut}/")) {
            return true;
        }
    }
    false
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledVersion {
    pub hash: String,
    pub path: String,
    pub has_player: bool,
    pub has_studio: bool,
    pub size_bytes: u64,
}

fn dir_size(p: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(rd) = std::fs::read_dir(p) {
        for entry in rd.flatten() {
            let child = entry.path();
            if child.is_dir() {
                total += dir_size(&child);
            } else if let Ok(md) = entry.metadata() {
                total += md.len();
            }
        }
    }
    total
}

pub fn clear_dir_contents(dir: &Path) -> Result<u64, String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(0),
    };
    let mut removed = 0u64;
    for entry in entries.flatten() {
        let p = entry.path();
        let is_dir = p.is_dir();
        removed += if is_dir {
            dir_size(&p)
        } else {
            entry.metadata().map(|m| m.len()).unwrap_or(0)
        };
        let res = if is_dir {
            std::fs::remove_dir_all(&p)
        } else {
            std::fs::remove_file(&p)
        };
        res.map_err(|e| format!("Không xóa được {}: {e}", p.display()))?;
    }
    Ok(removed)
}

pub fn scan(with_sizes: bool) -> Result<Vec<InstalledVersion>, String> {
    let root = versions_root();
    let rd = match std::fs::read_dir(&root) {
        Ok(rd) => rd,
        Err(_) => return Ok(Vec::new()),
    };
    let mut out = Vec::new();
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if validate_hash(&name).as_deref() != Some(name.as_str()) {
            continue;
        }
        let p = entry.path();
        out.push(InstalledVersion {
            has_player: p.join("RobloxPlayerBeta.exe").is_file(),
            has_studio: p.join("RobloxStudioBeta.exe").is_file(),
            size_bytes: if with_sizes { dir_size(&p) } else { 0 },
            hash: name.clone(),
            path: p.to_string_lossy().to_string(),
        });
    }
    out.sort_by(|a, b| a.hash.cmp(&b.hash));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_system_dirs() {
        assert!(is_unsafe_dir(Path::new("C:\\Windows")));
        assert!(is_unsafe_dir(Path::new("C:\\Windows\\System32")));
        assert!(is_unsafe_dir(Path::new("C:\\Program Files")));
        assert!(is_unsafe_dir(Path::new("C:\\")));
        assert!(is_unsafe_dir(Path::new("/usr/local")));
        assert!(is_unsafe_dir(Path::new("/etc")));
        assert!(is_unsafe_dir(Path::new("/")));
    }

    #[test]
    fn allows_safe_dirs() {
        assert!(!is_unsafe_dir(Path::new(
            "C:\\Users\\Long\\Desktop\\SomeFolder"
        )));
        assert!(!is_unsafe_dir(Path::new("C:\\Program Files\\Roblox")));
        assert!(!is_unsafe_dir(Path::new("D:\\Games\\RobloxVersions")));
    }

    #[test]
    fn clear_dir_contents_counts_before_deleting() {
        let dir = std::env::temp_dir().join("verlox-clear-cache-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("pkg.zip"), vec![0u8; 100]).unwrap();
        std::fs::write(dir.join("sub").join("inner.zip"), vec![0u8; 50]).unwrap();
        let removed = clear_dir_contents(&dir).unwrap();
        assert_eq!(removed, 150);
        assert!(std::fs::read_dir(&dir).unwrap().next().is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn clear_dir_contents_returns_zero_when_missing() {
        let dir = std::env::temp_dir().join("verlox-clear-cache-missing");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(clear_dir_contents(&dir).unwrap(), 0);
    }
}
