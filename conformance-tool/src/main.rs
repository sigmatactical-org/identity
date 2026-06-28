mod client;
mod config;
mod flow;
mod runner;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde_json::Value;
use tracing_subscriber::EnvFilter;

use client::ConformanceClient;
use config::{bootstrap_state_path, default_variant, load_config, plans_path, state_path};
use runner::{config_path, print_summary, run_plan};

#[derive(Parser)]
#[command(name = "sigma-conformance")]
#[command(about = "OpenID Connect RP conformance runner for sigma-identity")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Register the fake OP issuer with the conformance suite.
    Bootstrap {
        #[arg(long, default_value = "https://localhost.emobix.co.uk:8443")]
        conformance_server: String,
        #[arg(long, default_value = "sigma-identity-conformance")]
        client_id: String,
        #[arg(long, default_value = "conformance-client-secret-not-for-production")]
        client_secret: String,
    },
    /// Run one or more conformance plans.
    Run {
        #[arg(long, default_value = "https://localhost.emobix.co.uk:8443")]
        conformance_server: String,
        #[arg(long, default_value = "sigma-identity-conformance")]
        client_id: String,
        #[arg(long, default_value = "conformance-client-secret-not-for-production")]
        client_secret: String,
        #[arg(long, default_value = "dev")]
        version: String,
        #[arg(long, default_value_t = 300)]
        module_timeout: u64,
        #[arg(long)]
        plan: Option<String>,
        #[arg(long)]
        module: Vec<String>,
        #[arg(long)]
        all: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    // Environment mutation is process-local for the conformance runner only.
    unsafe {
        std::env::set_var("CONFORMANCE_ENSURE_IDENTITY", "1");
        std::env::set_var(
            "CONFORMANCE_IDENTITY_START_CMD",
            std::env::var("CONFORMANCE_IDENTITY_START_CMD").unwrap_or_else(|_| {
                "docker compose -f conformance/docker-compose.yml exec -d identity bash -lc \
                'pkill -x sigma-identity 2>/dev/null || true; \
                for _ in 1 2 3 4 5 6 7 8 9 10; do pgrep -x sigma-identity >/dev/null || break; sleep 1; done; \
                cd /workspace && cp .env.conformance-run .env 2>/dev/null || cp .env.conformance-ci .env; \
                nohup ./target/release/sigma-identity >/tmp/sigma-identity.log 2>&1 &'"
                    .into()
            }),
        );
    }

    match Cli::parse().command {
        Command::Bootstrap {
            conformance_server,
            client_id,
            client_secret,
        } => bootstrap(&conformance_server, &client_id, &client_secret).await,
        Command::Run {
            conformance_server,
            client_id,
            client_secret,
            version,
            module_timeout,
            plan,
            module,
            all,
        } => {
            run(
                &conformance_server,
                &client_id,
                &client_secret,
                &version,
                module_timeout,
                plan,
                module,
                all,
            )
            .await
        }
    }
}

async fn bootstrap(server: &str, client_id: &str, client_secret: &str) -> Result<()> {
    let client = ConformanceClient::new(server)?;
    client
        .wait_for_server(std::time::Duration::from_secs(180))
        .await?;

    let version = std::env::var("CONFORMANCE_VERSION").unwrap_or_else(|_| "dev".into());
    let config_file = config_path("config/oidcc-client-test.json");
    let (config, file_variant) = load_config(&config_file, client_id, client_secret, &version)?;
    let variant = file_variant.or_else(|| Some(default_variant()));

    let plan_id = client
        .create_test_plan("oidcc-client-test-plan", &config, variant.as_ref())
        .await?;
    let issuer = format!("{}/test/a/sigma-identity/", server.trim_end_matches('/'));
    let state = serde_json::json!({
        "plan_id": plan_id,
        "plan_name": "oidcc-client-test-plan",
        "issuer": issuer,
        "alias": "sigma-identity",
        "conformance_server": server,
    });
    std::fs::write(
        bootstrap_state_path(),
        serde_json::to_string_pretty(&state)? + "\n",
    )?;
    println!("{}", serde_json::to_string_pretty(&state)?);
    eprintln!("\nFake OP issuer ready: {issuer}");
    Ok(())
}

async fn run(
    server: &str,
    client_id: &str,
    client_secret: &str,
    version: &str,
    module_timeout: u64,
    plan: Option<String>,
    modules: Vec<String>,
    all: bool,
) -> Result<()> {
    if !all && plan.is_none() {
        bail!("Specify --all or --plan");
    }

    let client = ConformanceClient::new(server)?;
    client
        .wait_for_server(std::time::Duration::from_secs(180))
        .await?;

    let plans_doc: Value =
        serde_json::from_str(&std::fs::read_to_string(plans_path()).context("read plans.json")?)?;
    let all_plans = plans_doc
        .get("plans")
        .and_then(Value::as_array)
        .context("plans.json missing plans array")?;

    let selected: Vec<&Value> = if all {
        all_plans.iter().collect()
    } else {
        let plan_name = plan.unwrap();
        all_plans
            .iter()
            .filter(|entry| entry.get("name").and_then(Value::as_str) == Some(plan_name.as_str()))
            .collect::<Vec<_>>()
    };

    if selected.is_empty() {
        bail!("Unknown plan");
    }

    let only_modules = if modules.is_empty() {
        None
    } else {
        Some(modules.as_slice())
    };

    let mut aggregate = serde_json::json!({
        "plans": [],
        "conformance_server": server,
    });
    let mut all_ok = true;

    for entry in selected {
        let plan_name = entry
            .get("name")
            .and_then(Value::as_str)
            .context("plan entry missing name")?;
        let config_rel = entry
            .get("config")
            .and_then(Value::as_str)
            .context("plan entry missing config")?;
        let config_file = config_path(config_rel);
        let (config, file_variant) = load_config(&config_file, client_id, client_secret, version)?;
        let file_variant = file_variant.or_else(|| Some(default_variant()));
        let variant = entry
            .get("variant")
            .and_then(|v| v.as_object().cloned())
            .or(file_variant);

        let (plan_id, results, ok) = run_plan(
            &client,
            plan_name,
            &config,
            variant,
            module_timeout,
            only_modules,
        )
        .await?;
        print_summary(plan_name, &plan_id, &results, server);
        all_ok &= ok;

        let rows: Vec<Value> = results
            .iter()
            .map(|row| {
                serde_json::json!({
                    "name": row.name,
                    "result": row.result,
                    "module_id": row.module_id,
                })
            })
            .collect();
        aggregate["plans"]
            .as_array_mut()
            .expect("plans array")
            .push(serde_json::json!({
                "name": plan_name,
                "plan_id": plan_id,
                "results": rows,
                "ok": ok,
            }));
    }

    std::fs::write(
        state_path(),
        serde_json::to_string_pretty(&aggregate)? + "\n",
    )?;
    if all_ok {
        Ok(())
    } else {
        bail!("one or more conformance plans failed");
    }
}
