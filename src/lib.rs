//! lancer-promptfoo — a Rust reimplementation + native GUI of promptfoo, the
//! LLM red-team & evaluation framework. Part of the Lancer bug-bounty toolkit.
//!
//! Crate layout:
//! - [`core`]      — GUI-agnostic engine (config, registries, eval + red-team,
//!                   reporters, cache, self-test).
//! - [`cli`]       — full CLI surface (every upstream subcommand + flag).
//! - [`gui`]       — native egui desktop frontend (default invocation; `gui` feature).
//! - [`integration`] — subprocess wrapper around upstream promptfoo + HTTP capture shim.
//! - [`mcp`]       — Model Context Protocol server mode.
//! - [`share`] / [`auth`] / [`store`] — cloud sharing, auth, local eval store.

pub mod auth;
pub mod cli;
pub mod core;
pub mod integration;
pub mod mcp;
pub mod share;
pub mod store;

#[cfg(feature = "gui")]
pub mod gui;

use clap::Parser;

/// Initialize tracing/logging once. Safe to call multiple times.
pub fn init_tracing(verbose: bool) {
    let level = if verbose { "debug" } else { "info" };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| level.into()),
        )
        .with_target(false)
        .try_init();
}

/// Entry point for the primary `lancer-promptfoo` binary: open the GUI when
/// invoked with no subcommand (and the `gui` feature is built in), otherwise
/// run the CLI surface.
pub fn run_main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    init_tracing(cli.verbose);

    let rt = tokio::runtime::Runtime::new()?;

    let want_gui = cli.command.is_none() && !cli.cli && !cli.self_test && cfg!(feature = "gui");
    if want_gui {
        #[cfg(feature = "gui")]
        {
            // eframe runs the event loop on the main thread; the Tokio runtime
            // keeps its worker threads alive for spawned async work.
            return gui::launch(rt.handle().clone());
        }
    }

    rt.block_on(cli::run(cli))
}
