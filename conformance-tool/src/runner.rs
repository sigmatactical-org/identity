mod module_result;
pub use module_result::ModuleResult;

use std::sync::LazyLock;
use std::time::Duration;

use anyhow::Result;
use regex::Regex;
use serde_json::{Map, Value};
use tracing::{error, info, warn};

use crate::client::{ConformanceClient, format_module_log, is_passing};
use crate::config::{
    conformance_root, module_action, module_env_overrides, plan_env_overrides,
    write_conformance_env,
};
use crate::env::{env_f64, env_is_false};
use crate::flow::{ensure_identity_running, perform_module_action};

/// The suite's "variant already set by the plan" rejection, naming the variant.
static VARIANT_CONFLICT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"Variant (?:'|\\u0027)([^'\\]+)(?:'|\\u0027) has been set by user, but test plan already sets",
    )
    .expect("valid variant-conflict regex")
});

const TERMINAL_STATES: &[&str] = &[
    "INTERRUPTED",
    "FAILED",
    "WARNING",
    "REVIEW",
    "PASSED",
    "SKIPPED",
];

pub async fn create_test_plan_adaptive(
    client: &ConformanceClient,
    plan_name: &str,
    config: &Value,
    mut variant: Option<Map<String, Value>>,
) -> Result<String> {
    if variant.is_none() {
        return client.create_test_plan(plan_name, config, None).await;
    }

    loop {
        let current = variant.as_ref();
        match client.create_test_plan(plan_name, config, current).await {
            Ok(plan_id) => return Ok(plan_id),
            Err(err) => {
                let msg = format!("{err:#}");
                if let Some(caps) = VARIANT_CONFLICT.captures(&msg) {
                    let key = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    if let Some(map) = variant.as_mut()
                        && map.remove(key).is_some()
                    {
                        warn!("Plan {plan_name}: dropping variant {key} (already set by plan)");
                        continue;
                    }
                }
                return Err(err);
            }
        }
    }
}

async fn wait_for_module_settle(client: &ConformanceClient, module_id: &str) -> Result<String> {
    let settle = env_f64("CONFORMANCE_MODULE_SETTLE", 20.0);
    let deadline = tokio::time::Instant::now() + Duration::from_secs_f64(settle);
    let mut result = "UNKNOWN".to_string();
    while tokio::time::Instant::now() < deadline {
        let info = client.get_module_info(module_id).await?;
        if let Some(r) = info.get("result").and_then(Value::as_str) {
            result = r.to_string();
        }
        let status = info.get("status").and_then(Value::as_str).unwrap_or("");
        if is_passing(&result) || matches!(result.as_str(), "FAILED" | "INTERRUPTED") {
            break;
        }
        if matches!(status, "INTERRUPTED" | "FAILED") {
            break;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    tokio::time::sleep(Duration::from_secs_f64(settle)).await;
    let info = client.get_module_info(module_id).await?;
    Ok(info
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or(&result)
        .to_string())
}

pub async fn run_module(
    client: &ConformanceClient,
    plan_id: &str,
    module_name: &str,
    plan_name: &str,
    module_timeout: u64,
) -> Result<ModuleResult> {
    let module_id = client.start_test_module(plan_id, module_name).await?;
    let action = module_action(module_name);
    let mut overrides = module_env_overrides(module_name);
    overrides.extend(plan_env_overrides(plan_name));
    write_conformance_env(&overrides)?;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(module_timeout);
    let mut identity_started = false;
    let mut waiting_actions = 0u32;
    let mut result = "UNKNOWN".to_string();

    if crate::config::ensure_identity() {
        let _ = client
            .wait_for_status(&module_id, "WAITING", Duration::from_secs(120))
            .await;
    }

    while tokio::time::Instant::now() < deadline {
        let info = client.get_module_info(&module_id).await?;
        let status = info.get("status").and_then(Value::as_str).unwrap_or("");
        if let Some(r) = info.get("result").and_then(Value::as_str) {
            result = r.to_string();
        }

        if TERMINAL_STATES.contains(&status)
            || is_passing(&result)
            || matches!(result.as_str(), "FAILED" | "INTERRUPTED")
        {
            break;
        }

        if status == "FINISHED" {
            tokio::time::sleep(Duration::from_secs(3)).await;
            let info = client.get_module_info(&module_id).await?;
            if let Some(r) = info.get("result").and_then(Value::as_str) {
                result = r.to_string();
            }
            if is_passing(&result) || matches!(result.as_str(), "FAILED" | "INTERRUPTED") {
                break;
            }
        }

        if status == "WAITING" && crate::config::ensure_identity() {
            if !identity_started {
                let restart = !env_is_false("CONFORMANCE_RESTART_IDENTITY");
                ensure_identity_running(restart).await?;
                identity_started = true;
                perform_module_action(action, module_name).await?;
                waiting_actions += 1;
            } else if action == "login"
                && module_name == "oidcc-client-test-signing-key-rotation"
                && waiting_actions < 2
            {
                tokio::time::sleep(Duration::from_secs(8)).await;
                perform_module_action(action, module_name).await?;
                waiting_actions += 1;
            }
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    if tokio::time::Instant::now() >= deadline && !is_passing(&result) {
        let info = client.get_module_info(&module_id).await?;
        result = info
            .get("result")
            .and_then(Value::as_str)
            .unwrap_or("FAILED")
            .to_string();
        if !is_passing(&result) {
            anyhow::bail!(
                "Module {module_id} timed out (last status: {})",
                info.get("status").and_then(Value::as_str).unwrap_or("")
            );
        }
    }

    result = wait_for_module_settle(client, &module_id).await?;
    Ok(ModuleResult {
        name: module_name.to_string(),
        result,
        module_id,
    })
}

pub async fn run_plan(
    client: &ConformanceClient,
    plan_name: &str,
    config: &Value,
    variant: Option<Map<String, Value>>,
    module_timeout: u64,
    only_modules: Option<&[String]>,
) -> Result<(String, Vec<ModuleResult>, bool)> {
    let plan_id = create_test_plan_adaptive(client, plan_name, config, variant).await?;
    let modules = client.get_plan_modules(&plan_id).await?;
    let modules: Vec<Value> = if let Some(only) = only_modules {
        modules
            .into_iter()
            .filter(|module| {
                let name = module
                    .get("testModule")
                    .or_else(|| module.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                only.iter().any(|wanted| wanted == name)
            })
            .collect()
    } else {
        modules
    };

    let mut results = Vec::new();
    for module in modules {
        let module_name = module
            .get("testModule")
            .or_else(|| module.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let row = match run_module(client, &plan_id, &module_name, plan_name, module_timeout).await
        {
            Ok(row) => row,
            Err(err) => {
                error!("{module_name}: {err:#}");
                ModuleResult {
                    name: module_name.clone(),
                    result: "FAILED".into(),
                    module_id: String::new(),
                }
            }
        };
        info!("[{}] {}", row.result, row.name);
        if !is_passing(&row.result)
            && !row.module_id.is_empty()
            && let Ok(log) = client.get_module_log(&row.module_id).await
        {
            let formatted = format_module_log(&log, true);
            if !formatted.is_empty() {
                println!("\n--- {module_name} ---\n{formatted}\n");
            }
        }
        results.push(row);
        let gap = env_f64("CONFORMANCE_MODULE_GAP", 8.0);
        tokio::time::sleep(Duration::from_secs_f64(gap)).await;
    }

    let ok = results.iter().all(|row| is_passing(&row.result));
    Ok((plan_id, results, ok))
}

pub fn print_summary(plan_name: &str, plan_id: &str, results: &[ModuleResult], server: &str) {
    println!("\n{}", "=".repeat(72));
    println!("Plan: {plan_name}");
    println!("UI:   {server}/plan-detail.html?plan={plan_id}");
    for row in results {
        let mark = if is_passing(&row.result) { "+" } else { "!" };
        println!("  [{mark}] {:<10} {}", row.result, row.name);
    }
    println!("{}", "=".repeat(72));
}

pub fn config_path(relative: &str) -> std::path::PathBuf {
    conformance_root().join(relative)
}
