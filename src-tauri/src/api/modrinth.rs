use crate::models::mod_meta::{ModrinthProject, ModrinthVersion, ModrinthSearchResult};
use reqwest::header::USER_AGENT;

const API_BASE: &str = "https://api.modrinth.com/v2";
const APP_USER_AGENT: &str = "SpringLauncher/1.0.0 (contact@springlauncher.com)";

pub async fn search_mods(
    query: &str,
    facets: Option<&str>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> anyhow::Result<ModrinthSearchResult> {
    let client = reqwest::Client::new();
    let mut request = client.get(format!("{}/search", API_BASE))
        .header(USER_AGENT, APP_USER_AGENT)
        .query(&[("query", query)]);

    if let Some(f) = facets {
        request = request.query(&[("facets", f)]);
    }
    if let Some(l) = limit {
        let l_str = l.to_string();
        request = request.query(&[("limit", &l_str)]);
    }
    if let Some(o) = offset {
        let o_str = o.to_string();
        request = request.query(&[("offset", &o_str)]);
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let err_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        log::error!("Modrinth API Error ({}): {}", status, err_text);
        return Err(anyhow::anyhow!("Modrinth API error {}: {}", status, err_text));
    }

    let res = response.json::<ModrinthSearchResult>().await?;
    Ok(res)
}

pub async fn get_project(id_or_slug: &str) -> anyhow::Result<ModrinthProject> {
    let client = reqwest::Client::new();
    let res = client.get(format!("{}/project/{}", API_BASE, id_or_slug))
        .header(USER_AGENT, APP_USER_AGENT)
        .send()
        .await?
        .json::<ModrinthProject>()
        .await?;
    Ok(res)
}

pub async fn get_project_versions(id_or_slug: &str, loaders: Option<Vec<String>>, game_versions: Option<Vec<String>>) -> anyhow::Result<Vec<ModrinthVersion>> {
    let client = reqwest::Client::new();
    let url = format!("{}/project/{}/version", API_BASE, id_or_slug);

    let mut request = client.get(url).header(USER_AGENT, APP_USER_AGENT);

    if let Some(l) = loaders {
        let loaders_json = serde_json::to_string(&l)?;
        request = request.query(&[("loaders", loaders_json)]);
    }
    if let Some(gv) = game_versions {
        let gv_json = serde_json::to_string(&gv)?;
        request = request.query(&[("game_versions", gv_json)]);
    }

    let response = request.send().await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to fetch versions: {}", response.status()));
    }

    let res = response.json::<Vec<ModrinthVersion>>().await?;
    Ok(res)
}

pub async fn get_version(id: &str) -> anyhow::Result<ModrinthVersion> {
    let client = reqwest::Client::new();
    let res = client.get(format!("{}/version/{}", API_BASE, id))
        .header(USER_AGENT, APP_USER_AGENT)
        .send()
        .await?
        .json::<ModrinthVersion>()
        .await?;
    Ok(res)
}
