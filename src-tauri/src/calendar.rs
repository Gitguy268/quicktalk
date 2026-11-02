use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::TcpListener,
    time::{Duration, Instant},
};
#[derive(Serialize, Deserialize)]
struct Tokens {
    access_token: String,
    refresh_token: Option<String>,
}
fn key(provider: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new("app.quicktalk.desktop", &format!("calendar-{provider}"))
        .map_err(|e| e.to_string())
}
fn endpoints(provider: &str) -> Result<(&'static str, &'static str, &'static str), String> {
    match provider {
        "google" => Ok((
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://oauth2.googleapis.com/token",
            "https://www.googleapis.com/auth/calendar.events.readonly",
        )),
        "microsoft" => Ok((
            "https://login.microsoftonline.com/common/oauth2/v2.0/authorize",
            "https://login.microsoftonline.com/common/oauth2/v2.0/token",
            "offline_access https://graph.microsoft.com/Calendars.Read",
        )),
        _ => Err("Unknown calendar provider".into()),
    }
}
pub fn connect(provider: &str, client_id: &str, secret: &str) -> Result<(), String> {
    if client_id.trim().is_empty() {
        return Err("Enter your registered desktop OAuth client ID first.".into());
    }
    let (auth, token, scope) = endpoints(provider)?;
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let redirect = format!(
        "http://127.0.0.1:{}/callback",
        listener.local_addr().unwrap().port()
    );
    let state = uuid::Uuid::new_v4().to_string();
    let verifier = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(verifier.as_bytes()));
    let mut url = reqwest::Url::parse(auth).unwrap();
    url.query_pairs_mut().extend_pairs([
        ("client_id", client_id),
        ("redirect_uri", &redirect),
        ("response_type", "code"),
        ("scope", scope),
        ("state", &state),
        ("code_challenge", &challenge),
        ("code_challenge_method", "S256"),
        ("access_type", "offline"),
        ("prompt", "consent"),
    ]);
    open::that(url.as_str()).map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(180);
    let code = loop {
        if Instant::now() > deadline {
            return Err("Calendar sign-in timed out. Please try again.".into());
        }
        if let Ok((mut stream, _)) = listener.accept() {
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .map_err(|e| e.to_string())?;
            let mut data = [0u8; 8192];
            let n = stream.read(&mut data).map_err(|e| e.to_string())?;
            let request = String::from_utf8_lossy(&data[..n]);
            let target = request.split_whitespace().nth(1).unwrap_or("/");
            let callback = reqwest::Url::parse(&format!("http://localhost{target}"))
                .map_err(|e| e.to_string())?;
            let params: std::collections::HashMap<String, String> =
                callback.query_pairs().into_owned().collect();
            if callback.path() != "/callback" || params.get("state") != Some(&state) {
                let _ = stream.write_all(
                    b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\nInvalid callback",
                );
                continue;
            }
            let _=stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nYou can return to QuickTalk.");
            if let Some(code) = params.get("code") {
                break code.clone();
            }
            return Err("Calendar connection was not approved.".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let mut form = vec![
        ("grant_type", "authorization_code"),
        ("code", &code),
        ("client_id", client_id),
        ("redirect_uri", &redirect),
        ("code_verifier", &verifier),
    ];
    if !secret.is_empty() {
        form.push(("client_secret", secret));
    }
    let tokens: Tokens = reqwest::blocking::Client::new()
        .post(token)
        .form(&form)
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;
    key(provider)?
        .set_password(&serde_json::to_string(&tokens).unwrap())
        .map_err(|e| e.to_string())?;
    if !secret.is_empty() {
        keyring::Entry::new(
            "app.quicktalk.desktop",
            &format!("calendar-secret-{provider}"),
        )
        .map_err(|e| e.to_string())?
        .set_password(secret)
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn disconnect(provider: &str) -> Result<(), String> {
    endpoints(provider)?;
    let _ = key(provider)?.delete_credential();
    if let Ok(k) = keyring::Entry::new(
        "app.quicktalk.desktop",
        &format!("calendar-secret-{provider}"),
    ) {
        let _ = k.delete_credential();
    }
    Ok(())
}
pub fn events(provider: &str, client_id: &str) -> Result<Value, String> {
    let (_, token, _) = endpoints(provider)?;
    let mut tokens: Tokens = serde_json::from_str(
        &key(provider)?
            .get_password()
            .map_err(|_| "Connect your calendar first.")?,
    )
    .map_err(|e| e.to_string())?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    if let Some(refresh) = tokens.refresh_token.clone() {
        let secret = keyring::Entry::new(
            "app.quicktalk.desktop",
            &format!("calendar-secret-{provider}"),
        )
        .ok()
        .and_then(|k| k.get_password().ok())
        .unwrap_or_default();
        let mut form = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh),
            ("client_id", client_id),
        ];
        if !secret.is_empty() {
            form.push(("client_secret", &secret));
        }
        let mut updated: Tokens = client
            .post(token)
            .form(&form)
            .send()
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|_| "Calendar authorization expired. Reconnect your calendar.")?
            .json()
            .map_err(|e| e.to_string())?;
        if updated.refresh_token.is_none() {
            updated.refresh_token = Some(refresh);
        }
        key(provider)?
            .set_password(&serde_json::to_string(&updated).unwrap())
            .map_err(|e| e.to_string())?;
        tokens = updated;
    }
    let start = chrono::Utc::now().to_rfc3339();
    let end = (chrono::Utc::now() + chrono::Duration::days(14)).to_rfc3339();
    let response = if provider == "google" {
        client
            .get("https://www.googleapis.com/calendar/v3/calendars/primary/events")
            .query(&[
                ("timeMin", start.as_str()),
                ("timeMax", end.as_str()),
                ("singleEvents", "true"),
                ("orderBy", "startTime"),
                ("maxResults", "100"),
            ])
    } else {
        client
            .get("https://graph.microsoft.com/v1.0/me/calendarView")
            .header("Prefer", "outlook.timezone=\"UTC\"")
            .query(&[
                ("startDateTime", start.as_str()),
                ("endDateTime", end.as_str()),
                ("$top", "100"),
                ("$orderby", "start/dateTime"),
            ])
    }
    .bearer_auth(tokens.access_token)
    .send()
    .map_err(|e| e.to_string())?
    .error_for_status()
    .map_err(|e| e.to_string())?;
    let data: Value = response.json().map_err(|e| e.to_string())?;
    let items = data[if provider == "google" {
        "items"
    } else {
        "value"
    }]
    .as_array()
    .cloned()
    .unwrap_or_default();
    Ok(Value::Array(items.into_iter().filter(|e|e["status"]!="cancelled"&&e["isCancelled"]!=true).map(|e|{let title=if provider=="google"{e["summary"].clone()}else{e["subject"].clone()};let start=e["start"]["dateTime"].as_str().or(e["start"]["date"].as_str()).unwrap_or("");json!({"id":e["id"],"title":title,"start":if provider=="microsoft"{format!("{}Z",start.trim_end_matches('Z'))}else{start.into()},"provider":provider})}).collect()))
}
