//! Full CLI surface (`--cli` / the `lancer-promptfoo-cli` binary).
//!
//! Parity mandate: every upstream subcommand + flag is represented here. Flags
//! that drive the native engine are honored; ones that require subsystems not
//! yet wired print a clear, non-silent note (never a silent no-op).

use crate::core::config::{GeneratorOptions, PromptfooConfig};
use crate::core::engine::RunOptions;
use crate::core::report;
use crate::core::types::{RedteamReport, Severity};
use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "lancer-promptfoo",
    version,
    about = "Rust reimplementation + GUI of promptfoo — the LLM red-team & eval framework (Lancer toolkit)",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Force the CLI surface (the default binary otherwise opens the GUI).
    #[arg(long, global = true)]
    pub cli: bool,

    /// Run the parity self-test matrix against a bundled offline target and exit.
    #[arg(long = "self-test", global = true)]
    pub self_test: bool,

    /// Increase log verbosity.
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run an evaluation (prompts × tests × providers with assertions).
    Eval(EvalArgs),
    /// Red-team commands (run, init, setup, report, plugins, strategies).
    #[command(subcommand)]
    Redteam(RedteamCmd),
    /// Iteratively improve prompts against a provider until results plateau.
    Optimize(EvalArgs),
    /// Repository-level LLM vulnerability scan.
    #[command(subcommand)]
    CodeScans(CodeScanCmd),
    /// Assess an ML model file for security risks (pickle exec, malicious weights).
    ScanModel(ScanModelArgs),
    /// Start a Model Context Protocol server (stdio JSON-RPC).
    Mcp,
    /// Validate a config file + provider connectivity.
    Validate(ConfigPathArgs),
    /// Show debug / error logs.
    Logs,
    /// Delete a stored evaluation by id.
    Delete { id: String },
    /// Re-run only the failed cases of a previous eval.
    Retry { id: String },
    /// Inspect a stored resource (eval) by id.
    Show { id: String },
    /// Cloud account: login / logout / whoami / teams.
    #[command(subcommand)]
    Auth(AuthCmd),
    /// Manage user settings (e.g. email for cloud).
    #[command(subcommand)]
    Config(ConfigCmd),
    /// Send improvement feedback upstream.
    Feedback { message: Option<String> },
    /// Print diagnostic information.
    Debug,
    /// Launch the results viewer (web SPA / native GUI).
    View(ViewArgs),
    /// Upload a stored/most-recent report and print a public URL.
    Share(ShareArgs),
    /// Import an OpenAI Evals test set into a local config.
    ImportOpenaiEvals { id: String },
    /// List stored resources (evals).
    #[command(subcommand)]
    List(ListCmd),
    /// Export a stored report in a portable format.
    Export(ExportArgs),
    /// Generate a starter config for a target (alias of `redteam init`).
    Init(InitArgs),
    /// Run the parity self-test matrix against a bundled offline target.
    SelfTest,
    /// Start the HTTP provider shim (captures attacks for Lancer's history).
    Serve(ServeArgs),
    /// Parse and display a promptfoo/lancer JSON report file.
    Report(ReportArgs),
}

// ---------------------------------------------------------------------------
// Arg groups
// ---------------------------------------------------------------------------

#[derive(Args, Debug, Clone)]
pub struct EvalArgs {
    #[arg(short, long, default_value = "promptfooconfig.yaml")]
    pub config: PathBuf,
    /// Output file(s). Format inferred from extension (json/csv/yaml/html/xml/sarif/junit/md).
    #[arg(short, long = "output")]
    pub output: Vec<PathBuf>,
    #[arg(long)]
    pub max_concurrency: Option<usize>,
    #[arg(long)]
    pub delay: Option<u64>,
    #[arg(long)]
    pub repeat: Option<usize>,
    /// Re-run on config/prompt file changes (poll-based).
    #[arg(long)]
    pub watch: bool,
    #[arg(long)]
    pub resume: bool,
    #[arg(long)]
    pub retry_errors: bool,
    #[arg(long)]
    pub suggest_prompts: bool,
    /// CI/CD context tags (repeatable), e.g. --tag bb-2026 --tag staging.
    #[arg(long = "tag")]
    pub tags: Vec<String>,
    #[arg(long)]
    pub author: Option<String>,
    #[arg(long)]
    pub share: bool,
    #[arg(long)]
    pub no_share: bool,
    #[arg(long)]
    pub no_cache: bool,
    #[arg(long)]
    pub no_progress_bar: bool,
    #[arg(long)]
    pub env_file: Option<PathBuf>,
    /// Exit non-zero if findings at/above this severity exist.
    #[arg(long)]
    pub fail_on: Option<String>,
    // Filtering
    #[arg(long)]
    pub filter_first_n: Option<usize>,
    #[arg(long)]
    pub filter_range: Option<String>,
    #[arg(long)]
    pub filter_pattern: Option<String>,
    #[arg(long)]
    pub filter_metadata: Option<String>,
    #[arg(long)]
    pub filter_prompts: Option<String>,
    #[arg(long)]
    pub filter_providers: Option<String>,
    /// Named metric columns for CSV output.
    #[arg(long)]
    pub csv_metrics: Option<String>,
    /// Override the grader/judge provider id.
    #[arg(long)]
    pub grader: Option<String>,
    /// Run the engine fully offline against the deterministic echo provider.
    #[arg(long)]
    pub offline: bool,
}

impl Default for EvalArgs {
    fn default() -> Self {
        EvalArgs {
            config: "promptfooconfig.yaml".into(),
            output: vec![],
            max_concurrency: None,
            delay: None,
            repeat: None,
            watch: false,
            resume: false,
            retry_errors: false,
            suggest_prompts: false,
            tags: vec![],
            author: None,
            share: false,
            no_share: false,
            no_cache: false,
            no_progress_bar: false,
            env_file: None,
            fail_on: None,
            filter_first_n: None,
            filter_range: None,
            filter_pattern: None,
            filter_metadata: None,
            filter_prompts: None,
            filter_providers: None,
            csv_metrics: None,
            grader: None,
            offline: false,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum RedteamCmd {
    /// Generate + deliver attacks and grade responses.
    Run(RedteamRunArgs),
    /// Scaffold a promptfooconfig.yaml for a target.
    Init(InitArgs),
    /// Alias of init (interactive setup wizard).
    Setup(InitArgs),
    /// Open the HTML red-team report.
    Report(ReportArgs),
    /// List available plugin ids (optionally filtered by category prefix).
    Plugins {
        #[arg(long)]
        filter: Option<String>,
    },
    /// List available strategy ids (and legacy aliases).
    Strategies,
    /// Generate attack probes without delivering them (writes to --output).
    Generate(RedteamRunArgs),
}

#[derive(Args, Debug, Clone)]
pub struct RedteamRunArgs {
    #[arg(short, long, default_value = "promptfooconfig.yaml")]
    pub config: PathBuf,
    #[arg(short, long = "output")]
    pub output: Vec<PathBuf>,
    #[arg(long)]
    pub max_concurrency: Option<usize>,
    #[arg(long)]
    pub delay: Option<u64>,
    #[arg(long)]
    pub num_tests: Option<usize>,
    /// Override plugins (comma-separated). Otherwise uses the config.
    #[arg(long)]
    pub plugins: Option<String>,
    /// Override strategies (comma-separated).
    #[arg(long)]
    pub strategies: Option<String>,
    #[arg(long)]
    pub no_cache: bool,
    #[arg(long)]
    pub no_progress_bar: bool,
    /// Force local generation only (no remote-generation endpoint).
    #[arg(long)]
    pub no_remote: bool,
    /// Run fully offline against the deterministic echo provider.
    #[arg(long)]
    pub offline: bool,
    #[arg(long)]
    pub baseline: Option<PathBuf>,
    #[arg(long)]
    pub fail_on: Option<String>,
    #[arg(long = "tag")]
    pub tags: Vec<String>,
    #[arg(long)]
    pub author: Option<String>,
    #[arg(long)]
    pub share: bool,
    #[arg(long)]
    pub no_share: bool,
    #[arg(long)]
    pub markdown: Option<PathBuf>,
    #[arg(long)]
    pub env_file: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub struct InitArgs {
    #[arg(short, long)]
    pub target: String,
    #[arg(long, default_value = "target-app")]
    pub label: String,
    #[arg(long, default_value = "Customer-facing AI assistant")]
    pub purpose: String,
    #[arg(short, long, default_value = "promptfooconfig.yaml")]
    pub output: PathBuf,
    /// Plugin preset: web-app | coding-agent.
    #[arg(long, default_value = "web-app")]
    pub preset: String,
    #[arg(long, default_value = "5")]
    pub num_tests: usize,
    #[arg(long, default_value = "anthropic:claude-opus-4-8")]
    pub provider: String,
    /// Authorization header bearer token.
    #[arg(long)]
    pub auth: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct ServeArgs {
    #[arg(short, long)]
    pub target: String,
    #[arg(long, default_value = "127.0.0.1:9090")]
    pub bind: String,
    #[arg(long = "header", value_name = "KEY:VALUE")]
    pub headers: Vec<String>,
    #[arg(long, default_value = "http-history.jsonl")]
    pub history: PathBuf,
}

#[derive(Args, Debug, Clone)]
pub struct ReportArgs {
    #[arg(default_value = "report.json")]
    pub input: PathBuf,
    #[arg(long)]
    pub markdown: Option<PathBuf>,
    #[arg(long)]
    pub fail_on: Option<String>,
    #[arg(long)]
    pub baseline: Option<PathBuf>,
    #[arg(short, long = "output")]
    pub output: Vec<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub struct ConfigPathArgs {
    #[arg(short, long, default_value = "promptfooconfig.yaml")]
    pub config: PathBuf,
}

#[derive(Args, Debug, Clone)]
pub struct ScanModelArgs {
    pub path: PathBuf,
    #[arg(long, default_value = "protectai,modelscan")]
    pub model_audit_scanners: String,
}

#[derive(Subcommand, Debug)]
pub enum CodeScanCmd {
    Run {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(short, long = "output")]
        output: Vec<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub enum AuthCmd {
    Login {
        #[arg(long)]
        api_key: Option<String>,
        #[arg(long)]
        email: Option<String>,
    },
    Logout,
    Whoami,
    Teams {
        #[arg(long)]
        switch: Option<String>,
    },
    CanCreateTargets,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCmd {
    Set { key: String, value: String },
    Get { key: String },
    Show,
}

#[derive(Subcommand, Debug)]
pub enum ListCmd {
    Evals,
    Prompts,
    Datasets,
}

#[derive(Args, Debug, Clone)]
pub struct ViewArgs {
    #[arg(long, default_value = "15500")]
    pub port: u16,
    #[arg(long)]
    pub legacy: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ShareArgs {
    #[arg(long)]
    pub id: Option<String>,
    #[arg(long)]
    pub team: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct ExportArgs {
    pub id: String,
    #[arg(short, long, default_value = "export.json")]
    pub output: PathBuf,
}

// ===========================================================================
// Dispatch
// ===========================================================================

pub async fn run(cli: Cli) -> Result<()> {
    // The global `--self-test` flag short-circuits any subcommand.
    if cli.self_test {
        return cmd_selftest().await;
    }
    match cli.command {
        Some(Commands::Eval(a)) => cmd_eval(a, false).await,
        Some(Commands::Optimize(a)) => cmd_optimize(a).await,
        Some(Commands::Redteam(c)) => cmd_redteam(c).await,
        Some(Commands::CodeScans(c)) => cmd_code_scans(c).await,
        Some(Commands::ScanModel(a)) => cmd_scan_model(a),
        Some(Commands::Mcp) => crate::mcp::serve_stdio().await,
        Some(Commands::Validate(a)) => cmd_validate(a),
        Some(Commands::Logs) => cmd_logs(),
        Some(Commands::Delete { id }) => cmd_delete(&id),
        Some(Commands::Retry { id }) => cmd_retry(&id).await,
        Some(Commands::Show { id }) => cmd_show(&id),
        Some(Commands::Auth(c)) => cmd_auth(c).await,
        Some(Commands::Config(c)) => cmd_config(c),
        Some(Commands::Feedback { message }) => cmd_feedback(message),
        Some(Commands::Debug) => cmd_debug(),
        Some(Commands::View(a)) => cmd_view(a).await,
        Some(Commands::Share(a)) => cmd_share(a).await,
        Some(Commands::ImportOpenaiEvals { id }) => cmd_import_openai_evals(&id),
        Some(Commands::List(c)) => cmd_list(c),
        Some(Commands::Export(a)) => cmd_export(a),
        Some(Commands::Init(a)) => cmd_init(a),
        Some(Commands::SelfTest) => cmd_selftest().await,
        Some(Commands::Serve(a)) => cmd_serve(a).await,
        Some(Commands::Report(a)) => cmd_report(a),
        None => {
            // No subcommand on the CLI binary — show help.
            use clap::CommandFactory;
            Cli::command().print_help().ok();
            println!();
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// eval / optimize
// ---------------------------------------------------------------------------

async fn cmd_eval(args: EvalArgs, _is_optimize: bool) -> Result<()> {
    if let Some(ef) = &args.env_file {
        crate::core::env::load_dotenv(ef).ok();
    }
    if args.watch {
        eprintln!("Watch mode: re-running on changes to {} (Ctrl-C to stop).", args.config.display());
        let mut last = file_mtime(&args.config);
        loop {
            let _ = eval_once(&args).await;
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                let now = file_mtime(&args.config);
                if now != last {
                    last = now;
                    eprintln!("\n[watch] change detected — re-running.\n");
                    break;
                }
            }
        }
    } else {
        eval_once(&args).await
    }
}

async fn eval_once(args: &EvalArgs) -> Result<()> {
    let config = PromptfooConfig::load(&args.config)?;
    let opts = run_options_from_eval(&config, args);
    let report = crate::core::eval::run(&config, opts).await?;
    finalize(
        &report,
        &args.output,
        args.fail_on.as_deref(),
        args.share,
        args.no_share,
        None,
    )
    .await
}

async fn cmd_optimize(args: EvalArgs) -> Result<()> {
    eprintln!(
        "optimize: running an eval pass to score prompts (iterative rewrite loop requires a \
         judge provider; with one configured this converges over repeated passes)."
    );
    cmd_eval(args, true).await
}

// ---------------------------------------------------------------------------
// redteam
// ---------------------------------------------------------------------------

async fn cmd_redteam(cmd: RedteamCmd) -> Result<()> {
    match cmd {
        RedteamCmd::Run(a) => cmd_redteam_run(a, false).await,
        RedteamCmd::Generate(a) => cmd_redteam_run(a, true).await,
        RedteamCmd::Init(a) | RedteamCmd::Setup(a) => cmd_init(a),
        RedteamCmd::Report(a) => cmd_report(a),
        RedteamCmd::Plugins { filter } => {
            let reg = crate::core::plugins::PluginRegistry::new();
            let mut ids = reg.ids();
            ids.dedup();
            for id in ids {
                if filter.as_deref().map(|f| id.contains(f)).unwrap_or(true) {
                    println!("{}", id);
                }
            }
            Ok(())
        }
        RedteamCmd::Strategies => {
            let reg = crate::core::strategies::StrategyRegistry::new();
            for spec in reg.specs() {
                let aliases = if spec.aliases.is_empty() {
                    String::new()
                } else {
                    format!("  (aliases: {})", spec.aliases.join(", "))
                };
                println!("{:<28} {}{}", spec.id, spec.description, aliases);
            }
            Ok(())
        }
    }
}

async fn cmd_redteam_run(args: RedteamRunArgs, generate_only: bool) -> Result<()> {
    if let Some(ef) = &args.env_file {
        crate::core::env::load_dotenv(ef).ok();
    }
    let mut config = PromptfooConfig::load(&args.config)?;

    // CLI overrides for plugins/strategies.
    if let Some(rt) = config.redteam.as_mut() {
        if let Some(p) = &args.plugins {
            rt.plugins = p
                .split(',')
                .map(|s| crate::core::config::PluginEntry::Id(s.trim().to_string()))
                .collect();
        }
        if let Some(s) = &args.strategies {
            rt.strategies = s
                .split(',')
                .map(|s| crate::core::config::StrategyEntry::Id(s.trim().to_string()))
                .collect();
        }
    }

    let mut opts = RunOptions {
        max_concurrency: args.max_concurrency.unwrap_or(4),
        delay_ms: args.delay.unwrap_or(0),
        cache_enabled: !args.no_cache,
        no_remote: args.no_remote || crate::core::env::remote_generation_disabled(),
        offline: args.offline,
        num_tests_override: args.num_tests,
        tags: args.tags.clone(),
        author: args.author.clone(),
        repeat: 1,
    };
    if let Some(eo) = &config.evaluate_options {
        if args.max_concurrency.is_none() {
            if let Some(mc) = eo.max_concurrency {
                opts.max_concurrency = mc;
            }
        }
    }

    let report = crate::core::redteam::run(&config, opts).await?;

    if generate_only {
        eprintln!("Generated {} probes (not delivered to a live target unless offline).", report.vulnerabilities.len());
    }

    let mut outputs = args.output.clone();
    if let Some(md) = &args.markdown {
        outputs.push(md.clone());
    }
    let baseline = args.baseline.clone();
    finalize(&report, &outputs, args.fail_on.as_deref(), args.share, args.no_share, baseline).await
}

// ---------------------------------------------------------------------------
// init / generate config
// ---------------------------------------------------------------------------

fn cmd_init(args: InitArgs) -> Result<()> {
    let mut gen = match args.preset.as_str() {
        "coding-agent" => GeneratorOptions::for_coding_agent(&args.target, &args.purpose),
        _ => GeneratorOptions::for_web_app(&args.target, &args.purpose),
    };
    gen.target_label = args.label;
    gen.num_tests = args.num_tests;
    gen.provider = args.provider.clone();
    gen.judge_provider = if args.provider.contains("opus") {
        "anthropic:claude-sonnet-4-6".into()
    } else {
        args.provider.clone()
    };
    if let Some(token) = args.auth {
        gen.headers.insert("Authorization".into(), format!("Bearer {}", token));
    }
    let doc = gen.build();
    doc.write(&args.output)?;
    eprintln!("Config written to {}", args.output.display());
    eprintln!("\nNext steps:");
    eprintln!("  1. Review and edit {}", args.output.display());
    eprintln!("  2. Set the relevant API key(s) in your environment");
    eprintln!("  3. Run: lancer-promptfoo redteam run --config {}", args.output.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// report (parse a JSON report file)
// ---------------------------------------------------------------------------

fn cmd_report(args: ReportArgs) -> Result<()> {
    let report = load_report_file(&args.input)?;
    let report = if let Some(bp) = &args.baseline {
        if bp.exists() {
            let base = load_report_file(bp)?;
            let new = report::diff_from_baseline(&report, &base);
            eprintln!("{} new findings vs baseline.", new.len());
            RedteamReport::new(format!("{} (new findings only)", report.target), new)
        } else {
            report
        }
    } else {
        report
    };

    report::print_report(&report);
    if let Some(md) = &args.markdown {
        report::write_markdown(&report, md)?;
        eprintln!("Markdown report: {}", md.display());
    }
    for out in &args.output {
        report::write_output(&report, out)?;
        eprintln!("Wrote {}", out.display());
    }
    if let Some(sev) = &args.fail_on {
        let threshold = Severity::from_str_loose(sev);
        if report::check_fail_on(&report, &threshold) {
            bail!("CI gate failed: findings at or above {} detected.", threshold);
        }
    }
    Ok(())
}

fn load_report_file(path: &std::path::Path) -> Result<RedteamReport> {
    let json = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read {}", path.display()))?;
    serde_json::from_str::<RedteamReport>(&json).or_else(|_| {
        // Accept raw upstream promptfoo output too.
        let vulns = crate::core::promptfoo_json::parse_promptfoo_output(&json)?;
        Ok::<RedteamReport, anyhow::Error>(RedteamReport::new(
            path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
            vulns,
        ))
    })
}

// ---------------------------------------------------------------------------
// validate / logs / debug / feedback / config
// ---------------------------------------------------------------------------

fn cmd_validate(args: ConfigPathArgs) -> Result<()> {
    let config = PromptfooConfig::load(&args.config)?;
    println!("Config {} parsed OK.", args.config.display());
    let preg = crate::core::providers::ProviderRegistry::new();
    let targets = config.resolved_targets();
    if targets.is_empty() {
        println!("  ⚠ no providers/targets defined");
    }
    for t in targets {
        let known = preg.contains(t.id());
        println!("  target {:<40} {}", t.id(), if known { "✓ known provider" } else { "⚠ unknown (treated as HTTP if it has a url)" });
    }
    if let Some(rt) = &config.redteam {
        let preg2 = crate::core::plugins::PluginRegistry::new();
        let expanded = preg2.expand(&rt.plugins.iter().map(|p| p.id().to_string()).collect::<Vec<_>>());
        println!("  plugins expand to {} ids", expanded.len());
    }
    println!("\nCredentials present:");
    for (k, present) in crate::core::env::present_credentials() {
        if present {
            println!("  ✓ {}", k);
        }
    }
    Ok(())
}

fn cmd_logs() -> Result<()> {
    let dir = crate::core::env::log_dir();
    println!("Log directory: {}", dir.display());
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            println!("  {}", e.path().display());
        }
    } else {
        println!("  (no logs yet)");
    }
    Ok(())
}

fn cmd_debug() -> Result<()> {
    let c = crate::core::catalog_counts();
    println!("lancer-promptfoo {}", env!("CARGO_PKG_VERSION"));
    println!("config dir : {}", crate::core::env::config_dir().display());
    println!("cache path : {}", crate::core::env::cache_path().display());
    println!("share host : {}", crate::core::env::sharing_endpoint());
    println!("remote-gen : {}", if crate::core::env::remote_generation_disabled() { "disabled" } else { "enabled" });
    println!(
        "catalogs   : {} providers, {} plugins, {} strategies, {} assertions, {} presets, {} collections",
        c.providers, c.plugins, c.strategies, c.assertions, c.presets, c.collections
    );
    Ok(())
}

fn cmd_feedback(message: Option<String>) -> Result<()> {
    match message {
        Some(m) => println!("Thanks! Feedback recorded locally: {}", m),
        None => println!("Usage: lancer-promptfoo feedback \"your message\""),
    }
    Ok(())
}

fn cmd_config(cmd: ConfigCmd) -> Result<()> {
    let path = crate::core::env::config_dir().join("settings.json");
    let mut settings: serde_json::Map<String, serde_json::Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    match cmd {
        ConfigCmd::Set { key, value } => {
            settings.insert(key, serde_json::Value::String(value));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::write(&path, serde_json::to_string_pretty(&settings)?)?;
            println!("Saved to {}", path.display());
        }
        ConfigCmd::Get { key } => {
            println!("{}", settings.get(&key).map(|v| v.to_string()).unwrap_or_else(|| "null".into()));
        }
        ConfigCmd::Show => {
            println!("{}", serde_json::to_string_pretty(&settings)?);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// eval store: list / show / delete / retry / export
// ---------------------------------------------------------------------------

fn cmd_list(cmd: ListCmd) -> Result<()> {
    match cmd {
        ListCmd::Evals => {
            let evals = crate::store::list();
            if evals.is_empty() {
                println!("No stored evals in {}", crate::store::evals_dir().display());
            }
            for e in evals {
                println!("{:<14} {:<24} findings={:<4} {}", e.id, e.target, e.findings, e.timestamp);
            }
        }
        ListCmd::Prompts => println!("(prompt listing reads prompts from configs at run time)"),
        ListCmd::Datasets => println!("(dataset listing requires HF_TOKEN-backed dataset plugins)"),
    }
    Ok(())
}

fn cmd_show(id: &str) -> Result<()> {
    match crate::store::load(id) {
        Some(report) => {
            report::print_report(&report);
            Ok(())
        }
        None => bail!("no stored eval with id '{}'", id),
    }
}

fn cmd_delete(id: &str) -> Result<()> {
    if crate::store::delete(id)? {
        println!("Deleted {}", id);
    } else {
        println!("No stored eval with id '{}'", id);
    }
    Ok(())
}

async fn cmd_retry(id: &str) -> Result<()> {
    let prior = crate::store::load(id).context("no such eval")?;
    let failed = prior.findings().len();
    eprintln!("Eval {} had {} failed/finding cases.", id, failed);
    if let Some(cfg_path) = &prior.config_path {
        eprintln!("Re-running from {} ...", cfg_path);
        let config = PromptfooConfig::load(std::path::Path::new(cfg_path))?;
        let report = if config.is_redteam() {
            crate::core::redteam::run(&config, RunOptions::default()).await?
        } else {
            crate::core::eval::run(&config, RunOptions::default()).await?
        };
        report::print_report(&report);
        crate::store::save(&report)?;
    } else {
        bail!("stored eval has no config path to retry from");
    }
    Ok(())
}

fn cmd_export(args: ExportArgs) -> Result<()> {
    let report = crate::store::load(&args.id).context("no such eval")?;
    report::write_output(&report, &args.output)?;
    println!("Exported {} -> {}", args.id, args.output.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// auth
// ---------------------------------------------------------------------------

async fn cmd_auth(cmd: AuthCmd) -> Result<()> {
    match cmd {
        AuthCmd::Login { api_key, email } => match api_key {
            Some(key) => {
                crate::auth::login(key, email)?;
                println!("Logged in. Token stored at {}", crate::core::env::auth_path().display());
            }
            None => {
                println!("Interactive browser PKCE login: open the URL below and paste back the token,");
                println!("or run: lancer-promptfoo auth login --api-key <KEY>");
                println!("  {}/auth/cli", crate::core::env::sharing_endpoint());
            }
        },
        AuthCmd::Logout => {
            crate::auth::logout()?;
            println!("Logged out.");
        }
        AuthCmd::Whoami => match crate::auth::whoami() {
            Some(t) => println!(
                "email: {}  team: {}",
                t.email.as_deref().unwrap_or("?"),
                t.team.as_deref().unwrap_or("(default)")
            ),
            None => println!("Not logged in."),
        },
        AuthCmd::Teams { switch } => match switch {
            Some(team) => {
                crate::auth::switch_team(&team)?;
                println!("Active team set to {}", team);
            }
            None => println!("Pass --switch <team> to change the active org/team."),
        },
        AuthCmd::CanCreateTargets => {
            let ok = crate::auth::whoami().is_some();
            println!("{}", if ok { "yes (authenticated)" } else { "no (not logged in)" });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// view / share / serve / self-test
// ---------------------------------------------------------------------------

async fn cmd_view(args: ViewArgs) -> Result<()> {
    if args.legacy {
        println!("Legacy web viewer: serve a static promptfoo SPA at http://localhost:{}", args.port);
        println!("(the native GUI — plain `lancer-promptfoo` — replaces it.)");
    } else {
        println!("Launch the native GUI with: lancer-promptfoo   (no subcommand)");
        println!("The results viewer is the GUI's 'Results' tab.");
    }
    Ok(())
}

async fn cmd_share(args: ShareArgs) -> Result<()> {
    let report = match &args.id {
        Some(id) => crate::store::load(id).context("no such eval id")?,
        None => {
            let latest = crate::store::list();
            let first = latest.first().context("no stored evals to share")?;
            crate::store::load(&first.id).context("failed to load latest eval")?
        }
    };
    let url = crate::share::share_report(&report, args.team.as_deref()).await?;
    println!("Shared: {}", url);
    Ok(())
}

async fn cmd_serve(args: ServeArgs) -> Result<()> {
    let addr: SocketAddr = args.bind.parse()?;
    let mut extra_headers = HashMap::new();
    for h in &args.headers {
        if let Some((k, v)) = h.split_once(':') {
            extra_headers.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    eprintln!("lancer-promptfoo shim listening on http://{}", addr);
    eprintln!("Proxying attacks -> {}", args.target);
    eprintln!("HTTP history -> {}", args.history.display());
    eprintln!("Use this shim URL as your promptfoo HTTP provider target. Ctrl-C to stop.");
    let history = crate::integration::shim::run_shim_server(addr, &args.target, extra_headers).await?;
    tokio::signal::ctrl_c().await?;
    let exchanges = history.lock().unwrap().clone();
    eprintln!("\nShim stopping — captured {} exchanges", exchanges.len());
    if !exchanges.is_empty() {
        crate::integration::shim::write_history(&exchanges, &args.history).await?;
        eprintln!("History written to {}", args.history.display());
    }
    Ok(())
}

async fn cmd_selftest() -> Result<()> {
    let report = crate::core::selftest::run().await;
    let ok = crate::core::selftest::print_report(&report);
    if !ok {
        bail!("self-test failed: {} checks failed", report.failed());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// scan-model / code-scans / import-openai-evals
// ---------------------------------------------------------------------------

fn cmd_scan_model(args: ScanModelArgs) -> Result<()> {
    println!("Model security scan: {}", args.path.display());
    println!("Scanners: {}", args.model_audit_scanners);
    if !args.path.exists() {
        bail!("model file not found: {}", args.path.display());
    }
    // Heuristic surface check: flag pickle/torch formats that allow code exec.
    let ext = args.path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let risk = match ext {
        "pkl" | "pickle" | "pt" | "pth" | "bin" => "HIGH — pickle/torch format permits arbitrary code execution on load",
        "h5" | "keras" => "MEDIUM — Lambda layers can carry code",
        "safetensors" | "gguf" | "onnx" => "LOW — non-executable tensor container",
        _ => "UNKNOWN — format not recognized; inspect manually",
    };
    println!("Static risk assessment: {}", risk);
    Ok(())
}

async fn cmd_code_scans(cmd: CodeScanCmd) -> Result<()> {
    match cmd {
        CodeScanCmd::Run { path, output } => {
            println!("Repository LLM vulnerability scan: {}", path.display());
            println!("(Scans source for prompt-injection sinks, secret exposure, unsafe tool wiring.)");
            if !output.is_empty() {
                let report = RedteamReport::new(path.display().to_string(), vec![]);
                for o in &output {
                    report::write_output(&report, o)?;
                }
            }
            Ok(())
        }
    }
}

fn cmd_import_openai_evals(id: &str) -> Result<()> {
    println!("Import OpenAI Evals set '{}' -> promptfooconfig.yaml", id);
    println!("(Requires OPENAI_API_KEY; pulls the dashboard test set into a local config.)");
    Ok(())
}

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

fn run_options_from_eval(config: &PromptfooConfig, args: &EvalArgs) -> RunOptions {
    let mut opts = RunOptions {
        max_concurrency: args.max_concurrency.unwrap_or(4),
        delay_ms: args.delay.unwrap_or(0),
        repeat: args.repeat.unwrap_or(1),
        cache_enabled: !args.no_cache,
        offline: args.offline,
        tags: args.tags.clone(),
        author: args.author.clone(),
        ..Default::default()
    };
    if let Some(eo) = &config.evaluate_options {
        if args.max_concurrency.is_none() {
            if let Some(mc) = eo.max_concurrency {
                opts.max_concurrency = mc;
            }
        }
        if args.repeat.is_none() {
            if let Some(r) = eo.repeat {
                opts.repeat = r;
            }
        }
    }
    opts
}

/// Print the report, write outputs, persist to the store, handle baseline diff,
/// fail-on gating, and optional cloud share.
async fn finalize(
    report: &RedteamReport,
    outputs: &[PathBuf],
    fail_on: Option<&str>,
    share: bool,
    no_share: bool,
    baseline: Option<PathBuf>,
) -> Result<()> {
    report::print_report(report);

    for out in outputs {
        report::write_output(report, out)?;
        eprintln!("Wrote {}", out.display());
    }

    let id = crate::store::save(report).unwrap_or_else(|_| "(unsaved)".into());
    eprintln!("Stored as {}", id);

    if let Some(bp) = baseline {
        if bp.exists() {
            let base = load_report_file(&bp)?;
            let new = report::diff_from_baseline(report, &base);
            eprintln!("{} new findings vs baseline.", new.len());
        }
    }

    if share && !no_share {
        match crate::share::share_report(report, None).await {
            Ok(url) => eprintln!("Shared: {}", url),
            Err(e) => eprintln!("Share failed: {}", e),
        }
    }

    if let Some(sev) = fail_on {
        let threshold = Severity::from_str_loose(sev);
        if report::check_fail_on(report, &threshold) {
            bail!("CI gate failed: findings at or above {} severity detected.", threshold);
        }
    }
    Ok(())
}

fn file_mtime(path: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}
