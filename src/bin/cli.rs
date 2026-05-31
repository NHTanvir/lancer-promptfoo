//! `lancer-promptfoo-cli` — headless companion binary. Never links the GUI;
//! always runs the CLI surface (CI / servers without a display).

use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = lancer_promptfoo::cli::Cli::parse();
    lancer_promptfoo::init_tracing(cli.verbose);
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(lancer_promptfoo::cli::run(cli))
}
