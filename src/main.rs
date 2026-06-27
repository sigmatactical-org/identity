use mimalloc::MiMalloc;
use sigma_identity::run;
use tracing::{error, trace};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt::init();
    trace!("Starting initialization");

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let code = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime")
        .block_on(async {
            match run().await {
                Ok(()) => 0,
                Err(e) => {
                    error!("Failed to run: {e:?}");
                    1
                }
            }
        });

    std::process::exit(code);
}
