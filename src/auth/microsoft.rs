//! Microsoft -> Xbox Live -> XSTS -> Minecraft services auth chain
//! (device-code flow, so no embedded browser or local redirect server needed).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;
use serde::Deserialize;

use super::{AuthState, MinecraftAccount};

/// Azure application (client) ID for the OAuth device-code flow.
///
/// Set at build time: `TERRA_MS_CLIENT_ID=<guid> cargo build`.
/// Registering the Azure app is free, but calling the Minecraft API with it
/// additionally requires approval from Mojang (see README).
pub const CLIENT_ID: Option<&str> = option_env!("TERRA_MS_CLIENT_ID");

const DEVICE_CODE_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const SCOPE: &str = "XboxLive.signin offline_access";

#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    interval: u64,
    expires_in: u64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct MinecraftProfile {
    id: String,
    name: String,
}

fn set_state(state: &Arc<Mutex<AuthState>>, ctx: &egui::Context, new: AuthState) {
    *state.lock().unwrap() = new;
    ctx.request_repaint();
}

pub async fn login(
    state: &Arc<Mutex<AuthState>>,
    ctx: &egui::Context,
) -> Result<MinecraftAccount, String> {
    let client_id = CLIENT_ID
        .filter(|id| !id.is_empty())
        .ok_or("Не задан TERRA_MS_CLIENT_ID — см. README, раздел про Azure-приложение")?;

    let http = reqwest::Client::new();

    // 1. Request a device code.
    set_state(state, ctx, AuthState::InProgress("Запрашиваю код входа…".into()));
    let device: DeviceCodeResponse = http
        .post(DEVICE_CODE_URL)
        .form(&[("client_id", client_id), ("scope", SCOPE)])
        .send()
        .await
        .map_err(|e| format!("Сеть: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Device code: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Ответ device code: {e}"))?;

    // 2. Show the code to the user and poll until they sign in.
    set_state(
        state,
        ctx,
        AuthState::WaitingForUser {
            verification_uri: device.verification_uri.clone(),
            user_code: device.user_code.clone(),
        },
    );
    let ms_token = poll_for_token(&http, client_id, &device).await?;

    // 3. Xbox Live.
    set_state(state, ctx, AuthState::InProgress("Вход в Xbox Live…".into()));
    let (xbl_token, user_hash) = xbox_live_auth(&http, &ms_token).await?;

    // 4. XSTS.
    set_state(state, ctx, AuthState::InProgress("Получаю XSTS-токен…".into()));
    let xsts_token = xsts_auth(&http, &xbl_token).await?;

    // 5. Minecraft services.
    set_state(state, ctx, AuthState::InProgress("Вход в Minecraft…".into()));
    let mc_token = minecraft_auth(&http, &user_hash, &xsts_token).await?;

    // 6. Profile (username + UUID).
    set_state(state, ctx, AuthState::InProgress("Загружаю профиль…".into()));
    let profile = minecraft_profile(&http, &mc_token).await?;

    Ok(MinecraftAccount {
        username: profile.name,
        uuid: profile.id,
        access_token: mc_token,
    })
}

async fn poll_for_token(
    http: &reqwest::Client,
    client_id: &str,
    device: &DeviceCodeResponse,
) -> Result<String, String> {
    let mut interval = device.interval.max(1);
    let deadline = std::time::Instant::now() + Duration::from_secs(device.expires_in);
    loop {
        if std::time::Instant::now() > deadline {
            return Err("Код входа истёк — попробуй ещё раз".into());
        }
        tokio::time::sleep(Duration::from_secs(interval)).await;
        let resp: TokenResponse = http
            .post(TOKEN_URL)
            .form(&[
                ("client_id", client_id),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", device.device_code.as_str()),
            ])
            .send()
            .await
            .map_err(|e| format!("Сеть: {e}"))?
            .json()
            .await
            .map_err(|e| format!("Ответ токена: {e}"))?;
        if let Some(token) = resp.access_token {
            return Ok(token);
        }
        match resp.error.as_deref() {
            Some("authorization_pending") => continue,
            Some("slow_down") => interval += 5,
            Some("expired_token") => return Err("Код входа истёк — попробуй ещё раз".into()),
            Some(other) => return Err(format!("OAuth: {other}")),
            None => return Err("Пустой ответ от Microsoft".into()),
        }
    }
}

async fn xbox_live_auth(
    http: &reqwest::Client,
    ms_token: &str,
) -> Result<(String, String), String> {
    let body = serde_json::json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={ms_token}"),
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT",
    });
    let resp: serde_json::Value = http
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Сеть (XBL): {e}"))?
        .error_for_status()
        .map_err(|e| format!("Xbox Live: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Ответ XBL: {e}"))?;
    let token = resp["Token"]
        .as_str()
        .ok_or("XBL: нет токена в ответе")?
        .to_string();
    let user_hash = resp["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .ok_or("XBL: нет user hash в ответе")?
        .to_string();
    Ok((token, user_hash))
}

async fn xsts_auth(http: &reqwest::Client, xbl_token: &str) -> Result<String, String> {
    let body = serde_json::json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbl_token],
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT",
    });
    let resp = http
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Сеть (XSTS): {e}"))?;
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        let err: serde_json::Value = resp.json().await.unwrap_or_default();
        return Err(match err["XErr"].as_u64() {
            Some(2148916233) => {
                "У этого Microsoft-аккаунта нет Xbox-профиля — зайди один раз на xbox.com".into()
            }
            Some(2148916238) => {
                "Детский аккаунт: сначала добавь его в семейную группу Microsoft".into()
            }
            _ => format!("XSTS отказал: {err}"),
        });
    }
    let resp: serde_json::Value = resp
        .error_for_status()
        .map_err(|e| format!("XSTS: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Ответ XSTS: {e}"))?;
    resp["Token"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "XSTS: нет токена в ответе".into())
}

async fn minecraft_auth(
    http: &reqwest::Client,
    user_hash: &str,
    xsts_token: &str,
) -> Result<String, String> {
    let body = serde_json::json!({
        "identityToken": format!("XBL3.0 x={user_hash};{xsts_token}"),
    });
    let resp: serde_json::Value = http
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Сеть (Minecraft): {e}"))?
        .error_for_status()
        .map_err(|e| format!("Minecraft services: {e} (client_id не одобрен Mojang?)"))?
        .json()
        .await
        .map_err(|e| format!("Ответ Minecraft: {e}"))?;
    resp["access_token"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "Minecraft: нет access_token в ответе".into())
}

async fn minecraft_profile(
    http: &reqwest::Client,
    mc_token: &str,
) -> Result<MinecraftProfile, String> {
    let resp = http
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(mc_token)
        .send()
        .await
        .map_err(|e| format!("Сеть (профиль): {e}"))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("На этом аккаунте не куплен Minecraft: Java Edition".into());
    }
    resp.error_for_status()
        .map_err(|e| format!("Профиль: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Ответ профиля: {e}"))
}
