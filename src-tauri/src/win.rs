use std::path::Path;
use windows_registry::CURRENT_USER;

const CLASSES_PATH: &str = "Software\\Classes";
const PROTOCOLS: &[&str] = &["roblox", "roblox-player"];
const MARKER_VALUE: &str = "verlox";

fn register_proto(base: &str, proto: &str, exe: &Path) -> Result<(), String> {
    let key = CURRENT_USER
        .create(format!("{base}\\{proto}"))
        .map_err(|e| format!("Khong tao duoc khoa registry {proto}: {e}"))?;
    key.set_string("", format!("URL:{proto} Protocol"))
        .map_err(|e| format!("Khong ghi duoc mo ta protocol {proto}: {e}"))?;
    key.set_string("URL Protocol", "")
        .map_err(|e| format!("Khong ghi duoc gia tri 'URL Protocol' ({proto}): {e}"))?;
    key.set_string(MARKER_VALUE, exe.to_string_lossy())
        .map_err(|e| format!("Khong ghi duoc marker Verlox ({proto}): {e}"))?;
    let cmd = key
        .create("shell\\open\\command")
        .map_err(|e| format!("Khong tao duoc shell\\open\\command ({proto}): {e}"))?;
    cmd.set_string("", format!("\"{}\" \"%1\"", exe.display()))
        .map_err(|e| format!("Khong ghi duoc lenh mo protocol ({proto}): {e}"))?;
    Ok(())
}

fn unregister_proto(base: &str, proto: &str) -> Result<(), String> {
    let path = format!("{base}\\{proto}");
    let Ok(key) = CURRENT_USER.open(&path) else {
        return Err(format!(
            "Protocol {proto} chua duoc dang ky — khong co gi de go."
        ));
    };
    if key.get_string(MARKER_VALUE).is_err() {
        return Err(format!(
            "Protocol {proto} khong do Verlox dang ky — khong go de tranh anh huong dang ky goc cua Roblox."
        ));
    }
    CURRENT_USER
        .remove_tree(&path)
        .map_err(|e| format!("Khong go duoc khoa registry {proto}: {e}"))
}

pub fn register_protocol(exe: &Path) -> Result<(), String> {
    if !exe.is_file() {
        return Err(format!("Khong tim thay {}", exe.display()));
    }
    for proto in PROTOCOLS {
        register_proto(CLASSES_PATH, proto, exe)?;
    }
    Ok(())
}

pub fn unregister_protocol() -> Result<(), String> {
    let mut errs = Vec::new();
    for proto in PROTOCOLS {
        if let Err(e) = unregister_proto(CLASSES_PATH, proto) {
            errs.push(e);
        }
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(tag: &str) -> String {
        let b = format!("Software\\Classes\\verlox-test-{tag}");
        let _ = CURRENT_USER.remove_tree(&b);
        b
    }

    fn fake_exe() -> std::path::PathBuf {
        std::env::temp_dir()
            .join("verlox-protocol-test")
            .join("RobloxPlayerBeta.exe")
    }

    #[test]
    fn register_writes_expected_values() {
        let b = base("values");
        register_proto(&b, "roblox", &fake_exe()).unwrap();
        let key = CURRENT_USER.open(format!("{b}\\roblox")).unwrap();
        assert_eq!(key.get_string("").unwrap(), "URL:roblox Protocol");
        assert_eq!(key.get_string("URL Protocol").unwrap(), "");
        assert_eq!(
            key.get_string(MARKER_VALUE).unwrap(),
            fake_exe().to_string_lossy()
        );
        let cmd = CURRENT_USER
            .open(format!("{b}\\roblox\\shell\\open\\command"))
            .unwrap();
        assert_eq!(
            cmd.get_string("").unwrap(),
            format!("\"{}\" \"%1\"", fake_exe().display())
        );
        let _ = CURRENT_USER.remove_tree(&b);
    }

    #[test]
    fn unregister_removes_registered_key() {
        let b = base("remove");
        register_proto(&b, "roblox", &fake_exe()).unwrap();
        unregister_proto(&b, "roblox").unwrap();
        assert!(CURRENT_USER.open(format!("{b}\\roblox")).is_err());
        let _ = CURRENT_USER.remove_tree(&b);
    }

    #[test]
    fn unregister_refuses_unmarked_key() {
        let b = base("unmarked");
        CURRENT_USER
            .create(format!("{b}\\roblox"))
            .unwrap()
            .set_string("", "URL:roblox Protocol")
            .unwrap();
        let err = unregister_proto(&b, "roblox").unwrap_err();
        assert!(err.contains("khong do Verlox dang ky"));
        assert!(CURRENT_USER.open(format!("{b}\\roblox")).is_ok());
        let _ = CURRENT_USER.remove_tree(&b);
    }

    #[test]
    fn unregister_missing_key_is_error() {
        let b = base("none");
        let err = unregister_proto(&b, "roblox").unwrap_err();
        assert!(err.contains("chua duoc dang ky"));
    }
}
