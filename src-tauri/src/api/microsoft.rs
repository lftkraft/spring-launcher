use serde::Deserialize;
use reqwest::Client;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use crate::models::profile::Profile;

const CLIENT_ID: &str = "00000000402b5328";
const REDIRECT_URI: &str = "https://login.live.com/oauth20_desktop.srf";
const AUTH_URL: &str = "https://login.live.com/oauth20_authorize.srf";
const TOKEN_URL: &str = "https://login.live.com/oauth20_token.srf";

#[derive(Debug, Deserialize)]
struct MicrosoftTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct XboxLiveResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: HashMap<String, Vec<HashMap<String, String>>>,
}

#[derive(Debug, Deserialize)]
struct MinecraftProfileResponse {
    id: String,
    name: String,
}

pub async fn login_with_webview(app_handle: tauri::AppHandle) -> Result<Profile> {
    let login_url = format!(
        "{}?client_id={}&response_type=code&redirect_uri={}&scope=XboxLive.signin%20offline_access&prompt=select_account",
        AUTH_URL, CLIENT_ID, REDIRECT_URI
    );

    let window = tauri::WebviewWindowBuilder::new(
        &app_handle,
        "ms-login-v3",
        tauri::WebviewUrl::External(login_url.parse().unwrap())
    )
    .title("Microsoft Login")
    .inner_size(500.0, 650.0)
    .always_on_top(true)
    .center()
    .build()?;

    let mut auth_code = None;
    while window.is_visible().unwrap_or(false) {
        if let Ok(url) = window.url() {
            let url_str = url.as_str();
            if url_str.contains("code=") {
                if let Ok(parsed_url) = url::Url::parse(url_str) {
                    auth_code = parsed_url.query_pairs().find(|(k, _)| k == "code").map(|(_, v)| v.into_owned());
                    if auth_code.is_some() { let _ = window.close(); break; }
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    let code = auth_code.ok_or_else(|| anyhow!("Cancelled"))?;
    let client = Client::new();

    // MS Token
    let mut params = HashMap::new();
    params.insert("client_id", CLIENT_ID);
    params.insert("code", &code);
    params.insert("grant_type", "authorization_code");
    params.insert("redirect_uri", REDIRECT_URI);
    let ms_res = client.post(TOKEN_URL).form(&params).send().await?.json::<MicrosoftTokenResponse>().await?;

    // Xbox Live
    let xbl_req = serde_json::json!({ "Properties": { "AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com", "RpsTicket": format!("d={}", ms_res.access_token) }, "RelyingParty": "http://auth.xboxlive.com", "TokenType": "JWT" });
    let xbl_res = client.post("https://user.auth.xboxlive.com/user/authenticate").json(&xbl_req).send().await?.json::<XboxLiveResponse>().await?;
    let user_hash = xbl_res.display_claims.get("xui").and_then(|v| v.get(0)).and_then(|m| m.get("uhs")).ok_or_else(|| anyhow!("No hash"))?;

    // XSTS
    let xsts_req = serde_json::json!({ "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl_res.token] }, "RelyingParty": "rp://api.minecraftservices.com/", "TokenType": "JWT" });
    let xsts_res = client.post("https://xsts.auth.xboxlive.com/xsts/authorize").json(&xsts_req).send().await?.json::<XboxLiveResponse>().await?;

    // Minecraft Auth
    let mc_auth_req = serde_json::json!({ "identityToken": format!("XBL3.0 x={};{}", user_hash, xsts_res.token) });
    let mc_auth_res = client.post("https://api.minecraftservices.com/authentication/login_with_xbox").json(&mc_auth_req).send().await?.json::<serde_json::Value>().await?;
    let mc_token = mc_auth_res["access_token"].as_str().ok_or_else(|| anyhow!("No MC token"))?.to_string();

    // Profile
    let profile_res = client.get("https://api.minecraftservices.com/minecraft/profile").header("Authorization", format!("Bearer {}", mc_token)).send().await?.json::<MinecraftProfileResponse>().await?;

    Ok(Profile {
        id: uuid::Uuid::new_v4().to_string(),
        name: profile_res.name,
        uuid: profile_res.id,
        profile_type: "microsoft".to_string(),
        access_token: mc_token,
    })
}