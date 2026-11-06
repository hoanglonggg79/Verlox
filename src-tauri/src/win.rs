use std::path::Path;
use windows_registry::CURRENT_USER;

const CLASSES_PATH: &str = "Software\\Classes";
const PROTOCOLS: &[&str] = &["roblox", "roblox-player"];
const MARKER_VALUE: &str = "verlox";

fn register_proto(base: &str, proto: &str, exe: &Path) -> Result<(), String> {
    let key = CURRENT_USER
        .create(format!("{base}\\{proto}"))
        .map_err(|e| format!("Không tạo được khóa registry {proto}: {e}"))?;
    key.set_string("", format!("URL:{proto} Protocol"))
        .map_err(|e| format!("Không ghi được mô tả protocol {proto}: {e}"))?;
    key.set_string("URL Protocol", "")
        .map_err(|e| format!("Không ghi được giá trị 'URL Protocol' ({proto}): {e}"))?;
    key.set_string(MARKER_VALUE, exe.to_string_lossy())
        .map_err(|e| format!("Không ghi được marker Verlox ({proto}): {e}"))?;
    let cmd = key
        .create("shell\\open\\command")
        .map_err(|e| format!("Không tạo được shell\\open\\command ({proto}): {e}"))?;
    cmd.set_string("", format!("\"{}\" \"%1\"", exe.display()))
        .map_err(|e| format!("Không ghi được lệnh mở protocol ({proto}): {e}"))?;
    Ok(())
}

fn unregister_proto(base: &str, proto: &str) -> Result<(), String> {
    let path = format!("{base}\\{proto}");
    let Ok(key) = CURRENT_USER.open(&path) else {
        return Err(format!(
            "Protocol {proto} chưa được đăng ký — không có gì để gỡ."
        ));
    };
    if key.get_string(MARKER_VALUE).is_err() {
        return Err(format!(
            "Protocol {proto} không do Verlox đăng ký — không gỡ để tránh ảnh hưởng đăng ký gốc của Roblox."
        ));
    }
    CURRENT_USER
        .remove_tree(&path)
        .map_err(|e| format!("Không gỡ được khóa registry {proto}: {e}"))
}

pub fn register_protocol(exe: &Path) -> Result<(), String> {
    if !exe.is_file() {
        return Err(format!("Không tìm thấy {}", exe.display()));
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
        assert!(err.contains("không do Verlox đăng ký"));
        assert!(CURRENT_USER.open(format!("{b}\\roblox")).is_ok());
        let _ = CURRENT_USER.remove_tree(&b);
    }

    #[test]
    fn unregister_missing_key_is_error() {
        let b = base("none");
        let err = unregister_proto(&b, "roblox").unwrap_err();
        assert!(err.contains("chưa được đăng ký"));
    }
}
