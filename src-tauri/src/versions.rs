use crate::hash::validate_hash;

pub const WEAO_CURRENT_API: &str = "https://weao.xyz/api/versions/current";
pub const WEAO_FUTURE_API: &str = "https://weao.xyz/api/versions/future";
pub const WEAO_PAST_API: &str = "https://weao.xyz/api/versions/past";
pub const WEAO_EXPLOITS_API: &str = "https://weao.xyz/api/status/exploits";
pub const WEAO_USER_AGENT: &str = "WEAO-3PService";
pub const UMI_EXECUTOR_VERSION_API: &str =
    "https://raw.githubusercontent.com/NguoiGiaAnThitTreCon/umi/refs/heads/main/umi.txt";
pub const UMI_RBX_VERSION_API: &str =
    "https://raw.githubusercontent.com/NguoiGiaAnThitTreCon/umi/refs/heads/main/version.txt";
pub const VERSION_API_PLAYER: &str =
    "https://clientsettingscdn.roblox.com/v2/client-version/WindowsPlayer";
pub const VERSION_API_STUDIO: &str =
    "https://clientsettingscdn.roblox.com/v2/client-version/WindowsStudio";

fn weao(client: &reqwest::Client, url: &str) -> reqwest::RequestBuilder {
    client
        .get(url)
        .header(reqwest::header::USER_AGENT, WEAO_USER_AGENT)
}

#[derive(serde::Deserialize, Default)]
struct WeaoVersions {
    #[serde(rename = "Windows", default)]
    windows: Option<String>,
    #[serde(rename = "Mac", default)]
    mac: Option<String>,
}

async fn fetch_official(client: &reqwest::Client, url: &str) -> Option<String> {
    #[derive(serde::Deserialize, Default)]
    struct Official {
        #[serde(rename = "clientVersionUpload", default)]
        client_version_upload: Option<String>,
    }
    if let Ok(resp) = client.get(url).send().await {
        if let Ok(data) = resp.json::<Official>().await {
            return data.client_version_upload;
        }
    }
    None
}

async fn fetch_raw_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("không kết nối được: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.text()
        .await
        .map_err(|e| format!("không đọc được nội dung: {e}"))
}

pub async fn fetch_latest(client: &reqwest::Client) -> Option<String> {
    if let Ok(resp) = weao(client, WEAO_CURRENT_API).send().await {
        if let Ok(data) = resp.json::<WeaoVersions>().await {
            if let Some(w) = data.windows {
                return Some(w);
            }
        }
    }
    fetch_official(client, VERSION_API_PLAYER).await
}

pub async fn fetch_future_windows(client: &reqwest::Client) -> Option<String> {
    if let Ok(resp) = weao(client, WEAO_FUTURE_API).send().await {
        if let Ok(data) = resp.json::<WeaoVersions>().await {
            return data.windows;
        }
    }
    None
}

pub async fn fetch_previous_pair(client: &reqwest::Client) -> (Option<String>, Option<String>) {
    if let Ok(resp) = weao(client, WEAO_PAST_API).send().await {
        if let Ok(data) = resp.json::<WeaoVersions>().await {
            return (data.windows, data.mac);
        }
    }
    (None, None)
}

pub async fn fetch_studio_latest(client: &reqwest::Client) -> Option<String> {
    fetch_official(client, VERSION_API_STUDIO).await
}

#[derive(serde::Deserialize, Default)]
struct WeaoExploitRaw {
    #[serde(default)]
    title: String,
    #[serde(default)]
    version: String,
    #[serde(rename = "rbxversion", default)]
    rbx_version: Option<String>,
    #[serde(rename = "updateStatus", default)]
    update_status: bool,
    #[serde(default)]
    platform: String,
    #[serde(default)]
    free: bool,
    cost: Option<serde_json::Value>,
    #[serde(rename = "keysystem", default)]
    key_system: bool,
    #[serde(default)]
    hidden: bool,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExecutorInfo {
    pub name: String,
    pub executor_version: String,
    pub rbx_version: String,
    pub update_status: bool,
    pub free: bool,
    pub cost: Option<String>,
    pub key_system: Option<bool>,
}

fn cost_to_string(v: Option<serde_json::Value>) -> Option<String> {
    match v? {
        serde_json::Value::String(s) => Some(s),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn filter_exploits(raw: Vec<WeaoExploitRaw>) -> Vec<ExecutorInfo> {
    let mut out: Vec<ExecutorInfo> = Vec::new();
    for e in raw {
        if e.hidden || !e.platform.eq_ignore_ascii_case("windows") {
            continue;
        }
        let Some(rbx_raw) = e.rbx_version.as_deref() else {
            continue;
        };
        let Some(rbx_version) = validate_hash(rbx_raw) else {
            continue;
        };
        out.push(ExecutorInfo {
            name: e.title,
            executor_version: e.version,
            rbx_version,
            update_status: e.update_status,
            free: e.free,
            cost: cost_to_string(e.cost),
            key_system: Some(e.key_system),
        });
    }
    out
}

pub async fn fetch_executors(client: &reqwest::Client) -> Result<Vec<ExecutorInfo>, String> {
    let resp = weao(client, WEAO_EXPLOITS_API)
        .send()
        .await
        .map_err(|e| format!("Không gọi được WEAO exploits API: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        if status.as_u16() == 429 {
            return Err("WEAO cấm quá nhiều request (429) — thử lại sau vài phút.".to_string());
        }
        return Err(format!("WEAO exploits API trả mã {status}"));
    }
    let raw: Vec<WeaoExploitRaw> = resp
        .json()
        .await
        .map_err(|e| format!("Dữ liệu WEAO exploits không hợp lệ: {e}"))?;
    Ok(filter_exploits(raw))
}

pub fn umi_executor_from_texts(
    executor_version: &str,
    rbx_text: &str,
    latest: Option<&str>,
) -> Result<ExecutorInfo, String> {
    let version = executor_version.trim();
    if version.is_empty() {
        return Err("File umi.txt rỗng — không đọc được version của Umi".to_string());
    }
    let rbx_version = validate_hash(rbx_text.trim())
        .ok_or_else(|| format!("Umi ghi version Roblox không hợp lệ: {}", rbx_text.trim()))?;
    Ok(ExecutorInfo {
        name: "Umi".to_string(),
        executor_version: version.to_string(),
        update_status: match latest {
            Some(latest) => latest == rbx_version,
            None => true,
        },
        rbx_version,
        free: true,
        cost: None,
        key_system: Some(false),
    })
}

pub async fn fetch_umi_executor(client: &reqwest::Client) -> Result<ExecutorInfo, String> {
    let ver_text = fetch_raw_text(client, UMI_EXECUTOR_VERSION_API)
        .await
        .map_err(|e| format!("Không lấy được umi.txt: {e}"))?;
    let rbx_text = fetch_raw_text(client, UMI_RBX_VERSION_API)
        .await
        .map_err(|e| format!("Không lấy được version.txt của Umi: {e}"))?;
    let latest = fetch_official(client, VERSION_API_PLAYER).await;
    umi_executor_from_texts(&ver_text, &rbx_text, latest.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_keeps_only_windows_visible_valid_rbxversion() {
        let raw = vec![
            WeaoExploitRaw {
                title: "Potassium".into(),
                version: "1.1".into(),
                rbx_version: Some("version-eb4648e9148d440c".into()),
                update_status: true,
                platform: "Windows".into(),
                free: true,
                cost: None,
                key_system: false,
                hidden: false,
            },
            WeaoExploitRaw {
                platform: "Android".into(),
                rbx_version: Some("version-eb4648e9148d440c".into()),
                ..WeaoExploitRaw {
                    title: "AndroidOnly".into(),
                    ..Default::default()
                }
            },
            WeaoExploitRaw {
                hidden: true,
                rbx_version: Some("version-eb4648e9148d440c".into()),
                ..WeaoExploitRaw {
                    title: "HiddenOne".into(),
                    ..Default::default()
                }
            },
            WeaoExploitRaw {
                rbx_version: Some("".into()),
                ..WeaoExploitRaw {
                    title: "NoRbx".into(),
                    ..Default::default()
                }
            },
            WeaoExploitRaw {
                rbx_version: Some("1.2.3.4567".into()),
                ..WeaoExploitRaw {
                    title: "BadHash".into(),
                    ..Default::default()
                }
            },
        ];
        let list = filter_exploits(raw);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Potassium");
        assert_eq!(list[0].rbx_version, "version-eb4648e9148d440c");
        assert!(list[0].update_status);
    }

    #[test]
    fn filter_maps_cost_and_keysystem() {
        let raw = vec![WeaoExploitRaw {
            title: "Paid".into(),
            version: "2.0".into(),
            rbx_version: Some("version-b67e7cbe150a4c59".into()),
            update_status: false,
            platform: "windows".into(),
            free: false,
            cost: Some(serde_json::json!(15)),
            key_system: true,
            hidden: false,
        }];
        let list = filter_exploits(raw);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].cost.as_deref(), Some("15"));
        assert_eq!(list[0].key_system, Some(true));
        assert!(!list[0].update_status);
    }

    #[test]
    fn parses_weao_exploits_payload() {
        let text = r#"[
            {"title":"Xeno","version":"2.7.5","rbxversion":"version-eb4648e9148d440c",
             "updateStatus":true,"detected":false,"platform":"Windows",
             "free":true,"cost":null,"keysystem":false,"hidden":false,
             "sunc":{"suncScrap":0,"suncKey":[]},"uncStatus":80,"extype":1,"_id":"x"},
            {"title":"Invisible","rbxversion":"version-eb4648e9148d440c","platform":"Windows","hidden":true}
        ]"#;
        let raw: Vec<WeaoExploitRaw> = serde_json::from_str(text).expect("parse ok");
        let list = filter_exploits(raw);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Xeno");
        assert_eq!(list[0].executor_version, "2.7.5");
        assert_eq!(list[0].rbx_version, "version-eb4648e9148d440c");
        assert_eq!(list[0].key_system, Some(false));
    }

    #[test]
    fn umi_from_texts_parses_and_compares_latest() {
        let ex = umi_executor_from_texts(
            "0.57\n",
            "version-f0ea5da9412f4e35\n",
            Some("version-f0ea5da9412f4e35"),
        )
        .expect("umi ok");
        assert_eq!(ex.name, "Umi");
        assert_eq!(ex.executor_version, "0.57");
        assert_eq!(ex.rbx_version, "version-f0ea5da9412f4e35");
        assert!(ex.update_status);
        assert_eq!(ex.key_system, Some(false));
        assert!(ex.free);
        assert_eq!(ex.cost, None);

        let stale = umi_executor_from_texts(
            "0.57",
            "version-f0ea5da9412f4e35",
            Some("version-eb4648e9148d440c"),
        )
        .expect("umi ok");
        assert!(!stale.update_status);

        let unknown =
            umi_executor_from_texts("0.57", "version-f0ea5da9412f4e35", None).expect("umi ok");
        assert!(unknown.update_status);
    }

    #[test]
    fn umi_from_texts_rejects_bad_payloads() {
        assert!(umi_executor_from_texts("", "version-f0ea5da9412f4e35", None).is_err());
        assert!(umi_executor_from_texts("   \n ", "version-f0ea5da9412f4e35", None).is_err());
        assert!(umi_executor_from_texts("0.57", "1.2.3.4567", None).is_err());
        assert!(umi_executor_from_texts("0.57", "version-abc123", None).is_err());
    }
}
