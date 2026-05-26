//! lancer-promptfoo
//! Rust clone of promptfoo. See plan.md (local, gitignored) for the spec.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "lancer-promptfoo", version, about = "Rust clone of promptfoo")]
struct Cli {}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let _cli = Cli::parse();
    println!("lancer-promptfoo - scaffold ready. Implement per plan.md.");
    Ok(())
}
