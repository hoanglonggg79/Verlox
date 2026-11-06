use serde::Serialize;

pub const APP_SETTINGS_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Settings>\n    <ContentFolder>content</ContentFolder>\n    <BaseUrl>http://www.roblox.com</BaseUrl>\n</Settings>\n";

pub const PACKAGE_MAP: &[(&str, &str)] = &[
    ("RobloxApp.zip", ""),
    ("redist.zip", ""),
    ("shaders.zip", "shaders"),
    ("ssl.zip", "ssl"),
    ("WebView2.zip", ""),
    ("WebView2RuntimeInstaller.zip", "__webview2_installer__"),
    ("content-avatar.zip", "content/avatar"),
    ("content-configs.zip", "content/configs"),
    ("content-fonts.zip", "content/fonts"),
    ("content-sky.zip", "content/sky"),
    ("content-sounds.zip", "content/sounds"),
    ("content-textures2.zip", "content/textures"),
    ("content-models.zip", "content/models"),
    ("content-platform-fonts.zip", "PlatformContent/pc/fonts"),
    (
        "content-platform-dictionaries.zip",
        "PlatformContent/pc/shared_compression_dictionaries",
    ),
    ("content-terrain.zip", "PlatformContent/pc/terrain"),
    ("content-textures3.zip", "PlatformContent/pc/textures"),
    ("extracontent-luapackages.zip", "ExtraContent/LuaPackages"),
    ("extracontent-translations.zip", "ExtraContent/translations"),
    ("extracontent-models.zip", "ExtraContent/models"),
    ("extracontent-textures.zip", "ExtraContent/textures"),
    ("extracontent-places.zip", "ExtraContent/places"),
];

pub fn package_target(name: &str) -> Option<&'static str> {
    PACKAGE_MAP
        .iter()
        .find(|(k, _)| *k == name)
        .map(|(_, v)| *v)
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Package {
    pub name: String,
    pub md5: String,
    pub compressed: u64,
    pub uncompressed: u64,
}

pub fn parse_manifest(text: &str) -> Vec<Package> {
    let lines: Vec<&str> = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    let mut start = 0usize;
    if let Some(first) = lines.first() {
        let is_marker = matches!(*first, "v0" | "v1" | "v2");
        let skipable = is_marker || (!first.contains('.') && !first.starts_with("version"));
        if skipable {
            start = 1;
        }
    }

    let mut packages = Vec::new();
    let mut i = start;
    while i + 3 < lines.len() {
        packages.push(Package {
            name: lines[i].to_string(),
            md5: lines[i + 1].to_string(),
            compressed: lines[i + 2].parse::<u64>().unwrap_or(0),
            uncompressed: lines[i + 3].parse::<u64>().unwrap_or(0),
        });
        i += 4;
    }
    packages
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "v0\nRobloxApp.zip\n0d61f8370c1c6d1a\n121000\n240000\nssl.zip\n9c729f5dc571ae12\n3400\n9000";

    #[test]
    fn parses_sample_manifest() {
        let pkgs = parse_manifest(SAMPLE);
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].name, "RobloxApp.zip");
        assert_eq!(pkgs[0].md5, "0d61f8370c1c6d1a");
        assert_eq!(pkgs[0].compressed, 121000);
        assert_eq!(pkgs[0].uncompressed, 240000);
        assert_eq!(pkgs[1].name, "ssl.zip");
        assert_eq!(pkgs[1].compressed, 3400);
    }

    #[test]
    fn parses_without_marker() {
        let pkgs = parse_manifest("shaders.zip\nabc\n10\n20");
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].name, "shaders.zip");
    }

    #[test]
    fn ignores_partial_tail() {
        let pkgs = parse_manifest("v0\nssl.zip\nabc\n10");
        assert!(pkgs.is_empty());
    }

    #[test]
    fn target_lookup() {
        assert_eq!(package_target("content-avatar.zip"), Some("content/avatar"));
        assert_eq!(package_target("RobloxApp.zip"), Some(""));
        assert_eq!(
            package_target("extracontent-luapackages.zip"),
            Some("ExtraContent/LuaPackages")
        );
        assert_eq!(package_target("unknown.zip"), None);
    }
}
