use crate::hash::validate_hash;

pub const WEAO_CURRENT_API: &str = "https://weao.xyz/api/versions/current";
pub const WEAO_FUTURE_API: &str = "https://weao.xyz/api/versions/future";
pub const WEAO_PAST_API: &str = "https://weao.xyz/api/versions/past";
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
