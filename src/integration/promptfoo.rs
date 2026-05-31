//! Subprocess wrapper around the upstream `promptfoo` CLI (the "wrap + extend"
//! path from the plan). Useful when the user wants the exact upstream engine
//! but Lancer's reporting / HTTP capture / OAST correlation on top.

use crate::core::promptfoo_json::parse_promptfoo_output;
use crate::core::types::RedteamReport;
use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, info};

pub struct PromptfooRunner {
    pub config_path: PathBuf,
    pub output_path: PathBuf,
    pub no_progress_bar: bool,
    pub env_file: Option<PathBuf>,
}

impl PromptfooRunner {
    pub fn new(config_path: impl Into<PathBuf>, output_path: impl Into<PathBuf>) -> Self {
        PromptfooRunner {
            config_path: config_path.into(),
            output_path: output_path.into(),
            no_progress_bar: true,
            env_file: None,
        }
    }

    /// Run `promptfoo redteam run` and parse its JSON output.
    pub async fn run_redteam(&self) -> Result<RedteamReport> {
        self.ensure_promptfoo().await?;
        let target = self.target_name();
        info!(
            "Spawning: promptfoo redteam run --config {} --output {}",
            self.config_path.display(),
            self.output_path.display()
        );
        let mut cmd = Command::new("npx");
        let mut args = vec![
            "promptfoo@latest".to_string(),
            "redteam".into(),
            "run".into(),
            "--config".into(),
            self.config_path.display().to_string(),
            "--output".into(),
            self.output_path.display().to_string(),
        ];
        if self.no_progress_bar {
            args.push("--no-progress-bar".into());
        }
        if let Some(env_file) = &self.env_file {
            args.push("--env-file".into());
            args.push(env_file.display().to_string());
        }
        cmd.args(&args);
        self.spawn_and_stream(cmd).await?;
        self.parse_output(target)
    }

    /// Run `promptfoo eval` and parse its JSON output.
    pub async fn run_eval(&self) -> Result<RedteamReport> {
        self.ensure_promptfoo().await?;
        let target = self.target_name();
        let mut cmd = Command::new("npx");
        let mut args = vec![
            "promptfoo@latest".to_string(),
            "eval".into(),
            "--config".into(),
            self.config_path.display().to_string(),
            "--output".into(),
            self.output_path.display().to_string(),
        ];
        if self.no_progress_bar {
            args.push("--no-progress-bar".into());
        }
        cmd.args(&args);
        self.spawn_and_stream(cmd).await?;
        self.parse_output(target)
    }

    fn target_name(&self) -> String {
        self.config_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".into())
    }

    async fn spawn_and_stream(&self, mut cmd: Command) -> Result<()> {
        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn promptfoo. Is Node.js installed?")?;
        if let Some(stdout) = child.stdout.take() {
            let mut reader = BufReader::new(stdout).lines();
            while let Some(line) = reader.next_line().await? {
                eprintln!("[promptfoo] {}", line);
            }
        }
        let status = child.wait().await.context("Error waiting for promptfoo")?;
        if !status.success() {
            debug!("promptfoo exited with {:?} (often indicates findings)", status.code());
        }
        Ok(())
    }

    fn parse_output(&self, target: String) -> Result<RedteamReport> {
        let json = std::fs::read_to_string(&self.output_path)
            .with_context(|| format!("Could not read promptfoo output {}", self.output_path.display()))?;
        let vulns = parse_promptfoo_output(&json)?;
        let mut report = RedteamReport::new(target, vulns);
        report.config_path = Some(self.config_path.display().to_string());
        report.output_path = Some(self.output_path.display().to_string());
        Ok(report)
    }

    async fn ensure_promptfoo(&self) -> Result<()> {
        let status = Command::new("npx")
            .args(["promptfoo@latest", "--version"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;
        match status {
            Ok(s) if s.success() => Ok(()),
            _ => {
                eprintln!("[lancer-promptfoo] promptfoo not found — attempting via npx --yes ...");
                let install = Command::new("npx")
                    .args(["--yes", "promptfoo@latest", "--version"])
                    .status()
                    .await;
                match install {
                    Ok(s) if s.success() => Ok(()),
                    _ => bail!(
                        "promptfoo is not available. Install Node.js (https://nodejs.org) then: npm install -g promptfoo"
                    ),
                }
            }
        }
    }
}
