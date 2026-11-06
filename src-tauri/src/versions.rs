use crate::hash::validate_hash;

pub const WEAO_CURRENT_API: &str = "https://weao.xyz/api/versions/current";
pub const WEAO_FUTURE_API: &str = "https://weao.xyz/api/versions/future";
pub const WEAO_PAST_API: &str = "https://weao.xyz/api/versions/past";
pub const WEAO_EXPLOITS_API: &str = "https://weao.xyz/api/status/exploits";
pub const WEAO_USER_AGENT: &str = "WEAO-3PService";
pub const VERSION_API_PLAYER: &str =
    "https://clientsettingscdn.roblox.com/v2/client-version/WindowsPlayer";
pub const VERSION_API_STUDIO: &str =
    "https://clientsettingscdn.roblox.com/v2/client-version/WindowsStudio";
pub const DEPLOY_HISTORY_URL: &str = "https://setup.rbxcdn.com/DeployHistory.txt";

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

pub async fn fetch_deploy_history(client: &reqwest::Client) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let Ok(resp) = client.get(DEPLOY_HISTORY_URL).send().await else {
        return out;
    };
    if !resp.status().is_success() {
        return out;
    }
    let Ok(text) = resp.text().await else {
        return out;
    };
    for line in text.lines() {
        if !line.contains("version-") {
            continue;
        }
        for part in line.split_whitespace() {
            if let Some(vh) = validate_hash(part) {
                if seen.insert(vh.clone()) {
                    out.push(vh);
                    if out.len() >= 30 {
                        return out;
                    }
                }
            }
        }
    }
    out
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
    detected: bool,
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
    pub detected: bool,
    pub free: bool,
    pub cost: Option<String>,
    pub key_system: bool,
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
            detected: e.detected,
            free: e.free,
            cost: cost_to_string(e.cost),
            key_system: e.key_system,
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
                detected: false,
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
            detected: true,
            platform: "windows".into(),
            free: false,
            cost: Some(serde_json::json!(15)),
            key_system: true,
            hidden: false,
        }];
        let list = filter_exploits(raw);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].cost.as_deref(), Some("15"));
        assert!(list[0].key_system);
        assert!(!list[0].update_status);
        assert!(list[0].detected);
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
    }
}
