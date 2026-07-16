use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};

/// Whether the runner should start/restart sigma-identity itself when a
/// module is waiting on the relying party. Defaults to on; set
/// `CONFORMANCE_ENSURE_IDENTITY=0` (or `false`) to drive identity manually.
pub fn ensure_identity() -> bool {
    !std::env::var("CONFORMANCE_ENSURE_IDENTITY")
        .map(|v| matches!(v.as_str(), "0" | "false" | "FALSE"))
        .unwrap_or(false)
}

/// Shell command that (re)starts sigma-identity for a conformance run:
/// `CONFORMANCE_IDENTITY_START_CMD` when set (`scripts/conformance-stack.sh`
/// exports it), otherwise the docker-compose default.
pub fn identity_start_cmd() -> String {
    std::env::var("CONFORMANCE_IDENTITY_START_CMD").unwrap_or_else(|_| {
        "docker compose -f conformance/docker-compose.yml exec -d identity bash -lc \
        'pkill -x sigma-identity 2>/dev/null || true; \
        for _ in 1 2 3 4 5 6 7 8 9 10; do pgrep -x sigma-identity >/dev/null || break; sleep 1; done; \
        cd /workspace && cp .env.conformance-run .env 2>/dev/null || cp .env.conformance-ci .env; \
        ./target/release/sigma-identity >>/tmp/sigma-identity.log 2>&1'"
            .into()
    })
}

pub fn default_variant() -> Map<String, Value> {
    json!({
        "client_auth_type": "client_secret_post",
        "response_type": "code",
        "client_registration": "static_client",
        "request_type": "plain_http_request",
        "response_mode": "default",
    })
    .as_object()
    .cloned()
    .expect("default variant object")
}

pub fn identity_root() -> PathBuf {
    if let Ok(root) = std::env::var("SIGMA_IDENTITY_ROOT") {
        let path = PathBuf::from(root);
        if path.is_dir() {
            return path;
        }
    }

    if let Ok(mut dir) = std::env::current_dir() {
        loop {
            if dir.join("conformance/plans.json").is_file() {
                return dir;
            }
            if !dir.pop() {
                break;
            }
        }
    }

    // Devcontainer builds compile in /workspace; fall back when cwd discovery fails.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

pub fn conformance_root() -> PathBuf {
    identity_root().join("conformance")
}

pub fn env_base_path() -> PathBuf {
    identity_root().join(".env.conformance-ci")
}

pub fn env_run_path() -> PathBuf {
    identity_root().join(".env.conformance-run")
}

pub fn plans_path() -> PathBuf {
    conformance_root().join("plans.json")
}

pub fn state_path() -> PathBuf {
    conformance_root().join(".last-run.json")
}

pub fn bootstrap_state_path() -> PathBuf {
    conformance_root().join(".bootstrap-plan.json")
}

const MANAGED_ENV_KEYS: &[&str] = &[
    "IDENTITY_OIDC_TOKEN_ENDPOINT_AUTH_METHOD",
    "IDENTITY_OIDC_USERINFO_METHOD",
    "IDENTITY_OIDC_WEBFINGER_RESOURCE",
    "IDENTITY_OIDC_JWKS_REFRESH_AFTER_USERINFO",
    "IDENTITY_OIDC_AUTHORIZE_RESPONSE_MODE",
    "IDENTITY_LOGOUT_SEND_ID_TOKEN_HINT",
];

pub fn load_config(
    config_path: &Path,
    client_id: &str,
    client_secret: &str,
    version: &str,
) -> Result<(Value, Option<Map<String, Value>>)> {
    let raw = std::fs::read_to_string(config_path)
        .with_context(|| format!("read {}", config_path.display()))?;
    let raw = substitute_placeholders(&raw, client_id, client_secret, version)?;
    let mut config: Value = serde_json::from_str(&raw)?;
    let variant = config
        .as_object_mut()
        .and_then(|obj| obj.remove("variant"))
        .and_then(|v| v.as_object().cloned());
    Ok((config, variant))
}

fn substitute_placeholders(
    raw: &str,
    client_id: &str,
    client_secret: &str,
    version: &str,
) -> Result<String> {
    Ok(raw
        .replace(
            "{CLIENT_ID}",
            serde_json::to_string(client_id)?.trim_matches('"'),
        )
        .replace(
            "{CLIENT_SECRET}",
            serde_json::to_string(client_secret)?.trim_matches('"'),
        )
        .replace(
            "{VERSION}",
            serde_json::to_string(version)?.trim_matches('"'),
        ))
}

pub fn write_conformance_env(overrides: &HashMap<String, String>) -> Result<()> {
    let base = env_base_path();
    if !base.is_file() {
        bail!("Missing base env file: {}", base.display());
    }
    let lines = std::fs::read_to_string(&base)?;
    let managed: HashSet<&str> = MANAGED_ENV_KEYS.iter().copied().collect();
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for line in lines.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            out.push(line.to_string());
            continue;
        }
        let key = line.split('=').next().unwrap_or("");
        if managed.contains(key) {
            if let Some(value) = overrides.get(key) {
                out.push(format!("{key}={value}"));
                seen.insert(key.to_string());
            }
            continue;
        }
        out.push(line.to_string());
    }

    for (key, value) in overrides {
        if !seen.contains(key) {
            out.push(format!("{key}={value}"));
        }
    }

    std::fs::write(env_run_path(), out.join("\n") + "\n")?;
    Ok(())
}

pub fn module_action(module_name: &str) -> &'static str {
    if module_name.contains("discovery") {
        "discover"
    } else if module_name.contains("refreshtoken")
        || module_name.starts_with("oidcc-client-test-refresh")
    {
        "refresh"
    } else if module_name.contains("logout") {
        "logout"
    } else {
        "login"
    }
}

pub fn module_env_overrides(module_name: &str) -> HashMap<String, String> {
    let mut overrides = HashMap::new();
    if module_name.contains("client-secret-basic") {
        overrides.insert(
            "IDENTITY_OIDC_TOKEN_ENDPOINT_AUTH_METHOD".into(),
            "client_secret_basic".into(),
        );
    } else if module_action(module_name) == "login" {
        overrides.insert(
            "IDENTITY_OIDC_TOKEN_ENDPOINT_AUTH_METHOD".into(),
            "client_secret_post".into(),
        );
    }

    if module_name.contains("userinfo-bearer-body") {
        overrides.insert("IDENTITY_OIDC_USERINFO_METHOD".into(), "body".into());
    } else {
        overrides.insert("IDENTITY_OIDC_USERINFO_METHOD".into(), "header".into());
    }

    if module_name == "oidcc-client-test-discovery-webfinger-acct" {
        overrides.insert(
            "IDENTITY_OIDC_WEBFINGER_RESOURCE".into(),
            format!("acct:sigma-identity.{module_name}@localhost.emobix.co.uk"),
        );
    } else if module_name == "oidcc-client-test-discovery-webfinger-url" {
        overrides.insert(
            "IDENTITY_OIDC_WEBFINGER_RESOURCE".into(),
            format!("https://localhost.emobix.co.uk:8443/sigma-identity/{module_name}"),
        );
    }

    if module_name.contains("rotation") {
        overrides.insert(
            "IDENTITY_OIDC_JWKS_REFRESH_AFTER_USERINFO".into(),
            "1".into(),
        );
    }

    if module_action(module_name) == "logout" {
        overrides.insert("IDENTITY_LOGOUT_SEND_ID_TOKEN_HINT".into(), "1".into());
    }

    overrides
}

pub fn plan_env_overrides(plan_name: &str) -> HashMap<String, String> {
    match plan_name {
        "oidcc-client-refreshtoken-test-plan" => HashMap::from([
            (
                "IDENTITY_OIDC_AUTHORIZE_RESPONSE_MODE".into(),
                "form_post".into(),
            ),
            (
                "IDENTITY_OIDC_TOKEN_ENDPOINT_AUTH_METHOD".into(),
                "client_secret_basic".into(),
            ),
        ]),
        "oidcc-client-basic-certification-test-plan" => HashMap::from([(
            "IDENTITY_OIDC_TOKEN_ENDPOINT_AUTH_METHOD".into(),
            "client_secret_basic".into(),
        )]),
        _ => HashMap::new(),
    }
}
