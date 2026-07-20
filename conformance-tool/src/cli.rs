//! Command-line interface definition.

mod command;
pub use command::Command;

use clap::Parser;

#[derive(Parser)]
#[command(name = "sigma-conformance")]
#[command(about = "OpenID Connect RP conformance runner for sigma-identity")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}
