use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use regex::Regex;
use tokio::process::Command;
use tracing::info;

use crate::config::env_run_path;

pub const DEFAULT_IDENTITY_URL: &str = "https://localhost:3000/app/up";
pub const DEFAULT_LOGIN_APP_URI: &str = "https://localhost:3000/app/up";

pub async fn curl(args: &[&str], fail_on_http_error: bool) -> Result<()> {
    let mut cmd = Command::new("curl");
    cmd.arg("-sk");
    if fail_on_http_error {
        cmd.arg("-f");
    }
    for arg in args {
        cmd.arg(arg);
    }
    let status = cmd.status().await.context("run curl")?;
    if fail_on_http_error && !status.success() {
        bail!("curl failed with status {status}");
    }
    Ok(())
}

pub async fn ensure_identity_running(restart: bool) -> Result<()> {
    let identity_url = std::env::var("CONFORMANCE_IDENTITY_URL")
        .unwrap_or_else(|_| DEFAULT_IDENTITY_URL.into())
        .trim_end_matches('/')
        .to_string();

    if !restart {
        if curl(&["-sf", &identity_url], true).await.is_ok() {
            info!("Identity already running at {identity_url}");
            return Ok(());
        }
    }

    let start_cmd = std::env::var("CONFORMANCE_IDENTITY_START_CMD")
        .context("CONFORMANCE_IDENTITY_START_CMD must be set (see scripts/conformance-stack.sh)")?;
    info!("Starting identity: {start_cmd}");
    let status = Command::new("bash")
        .arg("-lc")
        .arg(&start_cmd)
        .status()
        .await
        .context("start identity")?;
    if !status.success() {
        bail!("identity start command failed");
    }

    let delay = env_f64("CONFORMANCE_IDENTITY_START_DELAY", 3.0);
    tokio::time::sleep(Duration::from_secs_f64(delay)).await;

    let timeout = env_f64("CONFORMANCE_IDENTITY_TIMEOUT", 120.0);
    let deadline = tokio::time::Instant::now() + Duration::from_secs_f64(timeout);
    while tokio::time::Instant::now() < deadline {
        if curl(&["-sf", &identity_url], true).await.is_ok() {
            info!("Identity ready at {identity_url}");
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    dump_identity_debug().await;
    bail!("Identity did not become ready at {identity_url}");
}

async fn dump_identity_debug() {
    let Some(cmd) = std::env::var("CONFORMANCE_IDENTITY_DEBUG_CMD").ok() else {
        return;
    };
    let _ = Command::new("bash")
        .arg("-lc")
        .arg(&cmd)
        .status()
        .await;
}

pub async fn trigger_discover() -> Result<()> {
    let identity_origin = identity_origin();
    let url = format!("{identity_origin}/auth/conformance/discover");
    info!("Triggering RP discovery: {url}");
    curl(&[&url], false).await
}

fn conformance_form_post_enabled() -> bool {
    if std::env::var("IDENTITY_OIDC_AUTHORIZE_RESPONSE_MODE")
        .ok()
        .as_deref()
        == Some("form_post")
    {
        return true;
    }
    let path = env_run_path();
    if let Ok(content) = std::fs::read_to_string(path) {
        return content
            .lines()
            .any(|line| line.trim() == "IDENTITY_OIDC_AUTHORIZE_RESPONSE_MODE=form_post");
    }
    false
}

async fn submit_form_post_callback(html: &str, cookie_jar: &str) -> Result<()> {
    let action_re = Regex::new(r#"action="([^"]+)""#).unwrap();
    let field_re = Regex::new(r#"name="([^"]+)"\s+value="([^"]+)""#).unwrap();
    let action = action_re
        .captures(html)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .context("form_post authorization response missing form action")?;
    let fields: Vec<(String, String)> = field_re
        .captures_iter(html)
        .map(|caps| (caps[1].to_string(), caps[2].to_string()))
        .collect();
    if !fields.iter().any(|(k, _)| k == "code") {
        bail!("form_post authorization response missing authorization code");
    }
    let data = fields
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    run_curl_cookie_jar(&[
        "-skL", "-c", cookie_jar, "-b", cookie_jar, "-X", "POST", action, "-d", &data,
    ])
    .await?;
    Ok(())
}

pub async fn trigger_login_flow(scope: Option<&str>, cookie_jar: &str) -> Result<()> {
    let identity_origin = identity_origin();
    if Path::new(cookie_jar).is_file() {
        let _ = std::fs::remove_file(cookie_jar);
    }
    let scope = scope.map(str::to_string).unwrap_or_else(|| {
        std::env::var("CONFORMANCE_LOGIN_SCOPE")
            .unwrap_or_else(|_| "openid profile email offline_access".into())
    });
    let params = format!(
        "scope={}&redirect_uri={}&app_uri={}&state=conformance-{}",
        urlencoding::encode(&scope),
        urlencoding::encode(&format!("{identity_origin}/auth/callback")),
        urlencoding::encode(
            &std::env::var("CONFORMANCE_LOGIN_APP_URI")
                .unwrap_or_else(|_| DEFAULT_LOGIN_APP_URI.into()),
        ),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    );
    let login_url = format!("{identity_origin}/auth/login?{params}");
    info!("Triggering RP login flow");

    if conformance_form_post_enabled() {
        let output =
            run_curl_cookie_jar(&["-skL", "-c", cookie_jar, "-b", cookie_jar, &login_url]).await?;
        submit_form_post_callback(&output, cookie_jar).await
    } else {
        run_curl_cookie_jar(&[
            "-skL",
            "-c",
            cookie_jar,
            "-b",
            cookie_jar,
            "-o",
            "/dev/null",
            &login_url,
        ])
        .await
        .map(|_| ())
    }
}

pub async fn trigger_refresh_only(cookie_jar: &str) -> Result<()> {
    let identity_origin = identity_origin();
    let refresh_url = format!("{identity_origin}/auth/refresh");
    info!("Triggering RP refresh: POST {refresh_url}");
    run_curl_cookie_jar(&[
        "-sk",
        "-b",
        cookie_jar,
        "-c",
        cookie_jar,
        "-X",
        "POST",
        &refresh_url,
    ])
    .await
    .map(|_| ())
}

pub async fn trigger_refresh_flow(cookie_jar: &str) -> Result<()> {
    trigger_login_flow(Some("openid profile email offline_access"), cookie_jar).await?;
    let delay = env_f64("CONFORMANCE_REFRESH_DELAY", 5.0);
    tokio::time::sleep(Duration::from_secs_f64(delay)).await;
    trigger_refresh_only(cookie_jar).await
}

async fn submit_logout_confirmation(html: &str, cookie_jar: &str) -> Result<()> {
    let action_re = Regex::new(r#"action="([^"]+)""#).unwrap();
    let field_re = Regex::new(r#"name="([^"]+)"\s+value="([^"]+)""#).unwrap();
    let Some(action) = action_re
        .captures(html)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
    else {
        return Ok(());
    };
    let fields: Vec<(String, String)> = field_re
        .captures_iter(html)
        .map(|caps| (caps[1].to_string(), caps[2].to_string()))
        .collect();
    let mut args = vec![
        "-skL".to_string(),
        "-c".to_string(),
        cookie_jar.to_string(),
        "-b".to_string(),
        cookie_jar.to_string(),
        "-X".to_string(),
        "POST".to_string(),
        action.to_string(),
    ];
    if !fields.is_empty() {
        let data = fields
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        args.push("-d".into());
        args.push(data);
    }
    run_curl_cookie_jar(&args.iter().map(String::as_str).collect::<Vec<_>>())
        .await
        .map(|_| ())
}

pub async fn trigger_logout_flow() -> Result<()> {
    let identity_origin = identity_origin();
    let cookie_jar = std::env::var("CONFORMANCE_LOGIN_COOKIE_JAR")
        .unwrap_or_else(|_| "/tmp/sigma-conformance-cookies.txt".into());
    trigger_login_flow(Some("openid profile email"), &cookie_jar).await?;
    let params = format!(
        "app_uri={}&redirect_uri={}",
        urlencoding::encode(&format!("{identity_origin}/conformance/")),
        urlencoding::encode(&format!("{identity_origin}/auth/logoutcallback")),
    );
    let logout_url = format!("{identity_origin}/auth/logout?{params}");
    info!("Triggering RP logout flow");
    let html =
        run_curl_cookie_jar(&["-skL", "-b", &cookie_jar, "-c", &cookie_jar, &logout_url]).await?;
    let lower = html.to_lowercase();
    if lower.contains("form") && lower.contains("logout") {
        submit_logout_confirmation(&html, &cookie_jar).await?;
    }
    Ok(())
}

pub fn module_cookie_jar(module_name: &str) -> String {
    format!("/tmp/sigma-conformance-{module_name}.cookies.txt")
}

pub async fn perform_module_action(action: &str, module_name: &str) -> Result<()> {
    let cookie_jar = module_cookie_jar(module_name);
    match action {
        "discover" => trigger_discover().await,
        "refresh" => trigger_refresh_flow(&cookie_jar).await,
        "logout" => trigger_logout_flow().await,
        _ => {
            let scope = if module_name.contains("distributed-claims") {
                Some("openid profile email")
            } else {
                None
            };
            trigger_login_flow(scope, &cookie_jar).await
        }
    }
}

async fn run_curl_cookie_jar(args: &[&str]) -> Result<String> {
    let output = Command::new("curl")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("run curl")?;
    if !output.status.success() {
        bail!(
            "curl {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn identity_origin() -> String {
    std::env::var("CONFORMANCE_IDENTITY_ORIGIN").unwrap_or_else(|_| "https://localhost:3000".into())
}

fn env_f64(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
