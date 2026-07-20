//! [`Command`].

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Command {
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
