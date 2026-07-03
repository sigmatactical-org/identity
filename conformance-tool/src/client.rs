//! OpenID Foundation conformance suite API client.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde_json::Value;
use tracing::{debug, info};

pub const PASSING_RESULTS: &[&str] = &["PASSED", "WARNING", "REVIEW", "SKIPPED"];

pub struct ConformanceClient {
    client: Client,
    server: String,
    host_header: Option<String>,
}

impl ConformanceClient {
    pub fn new(server: &str) -> Result<Self> {
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .context("build HTTP client")?;
        let host_header = host_header_for(server);
        Ok(Self {
            client,
            server: server.trim_end_matches('/').to_string(),
            host_header,
        })
    }

    pub async fn wait_for_server(&self, timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            match self.get_raw("/").await {
                Ok(status) if status == 200 || status == 302 => {
                    info!("Conformance suite is up at {}", self.server);
                    return Ok(());
                }
                Err(err) => {
                    debug!("Waiting for conformance suite: {err:#}");
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
                Ok(status) if status == 401 || status == 403 => {
                    info!("Conformance suite is up at {} (HTTP {status})", self.server);
                    return Ok(());
                }
                Ok(_) => {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            }
        }
        bail!("Conformance suite not ready after {}s", timeout.as_secs());
    }

    pub async fn create_test_plan(
        &self,
        plan_name: &str,
        config: &Value,
        variant: Option<&serde_json::Map<String, Value>>,
    ) -> Result<String> {
        let mut url = format!(
            "{}/api/plan?planName={}",
            self.server,
            urlencoding::encode(plan_name)
        );
        if let Some(variant) = variant {
            let variant_json = serde_json::to_string(variant)?;
            let encoded = urlencoding::encode(&variant_json);
            url.push_str("&variant=");
            url.push_str(&encoded);
        }
        info!("Creating test plan {plan_name}");
        let response = self.post_json(&url, config).await?;
        let plan_id = response
            .get("id")
            .or_else(|| response.get("plan").and_then(|p| p.get("id")))
            .and_then(Value::as_str)
            .context(format!("No plan ID in create response: {response}"))?;
        info!("Created plan {plan_name} ({plan_id})");
        Ok(plan_id.to_string())
    }

    pub async fn get_plan_modules(&self, plan_id: &str) -> Result<Vec<Value>> {
        let plan = self.get_json(&format!("/api/plan/{plan_id}")).await?;
        let modules = plan
            .get("modules")
            .and_then(Value::as_array)
            .context(format!("Plan {plan_id} has no modules"))?;
        Ok(modules.clone())
    }

    pub async fn start_test_module(&self, plan_id: &str, module_name: &str) -> Result<String> {
        let url = format!(
            "{}/api/runner?plan={}&test={}",
            self.server,
            urlencoding::encode(plan_id),
            urlencoding::encode(module_name)
        );
        // Suite expects query params only (HTTP 201), not a JSON body.
        let data = self.post_no_body(&url).await?;
        data.get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .context(format!("No module ID in start response: {data}"))
    }

    pub async fn get_module_info(&self, module_id: &str) -> Result<Value> {
        self.get_json(&format!("/api/info/{module_id}")).await
    }

    pub async fn get_module_log(&self, module_id: &str) -> Result<Vec<Value>> {
        let log = self.get_json(&format!("/api/log/{module_id}")).await?;
        Ok(log.as_array().cloned().unwrap_or_default())
    }

    pub async fn wait_for_status(
        &self,
        module_id: &str,
        status: &str,
        timeout: Duration,
    ) -> Result<Value> {
        self.wait_for_state(module_id, &[status], timeout).await
    }

    pub async fn wait_for_state(
        &self,
        module_id: &str,
        terminal_states: &[&str],
        timeout: Duration,
    ) -> Result<Value> {
        let deadline = Instant::now() + timeout;
        loop {
            let info = self.get_module_info(module_id).await?;
            let status = info.get("status").and_then(Value::as_str).unwrap_or("");
            if terminal_states.iter().any(|s| *s == status) {
                return Ok(info);
            }
            if Instant::now() >= deadline {
                bail!("Module {module_id} timed out (last status: {status})");
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    async fn get_json(&self, path: &str) -> Result<Value> {
        let body = self.get_bytes(path, None).await?;
        Ok(serde_json::from_slice(&body)?)
    }

    async fn get_raw(&self, path: &str) -> Result<u16> {
        let url = self.build_url(path, None);
        let mut request = self.client.get(&url);
        if let Some(host) = &self.host_header {
            request = request.header("Host", host);
        }
        let response = request.send().await.context(format!("GET {path}"))?;
        Ok(response.status().as_u16())
    }

    async fn get_bytes(&self, path: &str, params: Option<&[(&str, &str)]>) -> Result<Vec<u8>> {
        let url = self.build_url(path, params);
        let mut request = self.client.get(&url);
        if let Some(host) = &self.host_header {
            request = request.header("Host", host);
        }
        let response = request.send().await.context(format!("GET {path}"))?;
        let status = response.status();
        let body = response.bytes().await?;
        if !status.is_success() {
            bail!(
                "GET {path} failed: HTTP {}: {}",
                status,
                String::from_utf8_lossy(&body)
            );
        }
        Ok(body.to_vec())
    }

    async fn post_no_body(&self, url: &str) -> Result<Value> {
        let mut request = self.client.post(url);
        if let Some(host) = &self.host_header {
            request = request.header("Host", host);
        }
        let response = request.send().await.context(format!("POST {url}"))?;
        let status = response.status();
        let bytes = response.bytes().await?;
        if !status.is_success() {
            bail!(
                "POST {url} failed: HTTP {}: {}",
                status,
                String::from_utf8_lossy(&bytes)
            );
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    async fn post_json(&self, url: &str, body: &Value) -> Result<Value> {
        let mut request = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(body);
        if let Some(host) = &self.host_header {
            request = request.header("Host", host);
        }
        let response = request.send().await.context(format!("POST {url}"))?;
        let status = response.status();
        let bytes = response.bytes().await?;
        if !status.is_success() {
            bail!(
                "POST {url} failed: HTTP {}: {}",
                status,
                String::from_utf8_lossy(&bytes)
            );
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn build_url(&self, path: &str, params: Option<&[(&str, &str)]>) -> String {
        let base = if self.server.contains("127.0.0.1") {
            "https://127.0.0.1:8443".to_string()
        } else {
            self.server.clone()
        };
        let mut url = format!("{base}{path}");
        if let Some(params) = params {
            let query = params
                .iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            url.push('?');
            url.push_str(&query);
        }
        url
    }
}

fn host_header_for(server: &str) -> Option<String> {
    if server.contains("127.0.0.1") {
        Some("localhost.emobix.co.uk".into())
    } else {
        None
    }
}

pub fn format_module_log(entries: &[Value], verbose: bool) -> String {
    let mut lines = Vec::new();
    for entry in entries {
        let result = entry.get("result").and_then(Value::as_str).unwrap_or("");
        let src = entry.get("src").and_then(Value::as_str).unwrap_or("");
        let msg = entry.get("msg").and_then(Value::as_str).unwrap_or("");
        if verbose || !matches!(result, "SUCCESS" | "INFO" | "") {
            lines.push(format!("[{result}] {src}: {msg}"));
        }
        if let Some(http) = entry.get("http") {
            lines.push(format!("  http: {http}"));
        }
    }
    lines.join("\n")
}

pub fn is_passing(result: &str) -> bool {
    PASSING_RESULTS.contains(&result)
}
