//! Native egui desktop frontend (the default `lancer-promptfoo` invocation).
//!
//! Three-pane Lancer-standard layout: left inputs/jobs, center findings table,
//! right detail pane, top menu bar, bottom status bar. All long-running work
//! runs on the Tokio runtime; the egui thread only renders and polls shared
//! state. Compiled only with the `gui` feature.

pub mod crosstool;

use crate::core::config::{GeneratorOptions, PromptfooConfig, ProviderConfig};
use crate::core::engine::RunOptions;
use crate::core::types::{Severity, Vulnerability};
use crosstool::SharedState;
use std::path::Path;

/// Launch the GUI. Starts the cross-tool server, then runs the egui event loop
/// on the main thread (winit requirement).
pub fn launch(handle: tokio::runtime::Handle) -> anyhow::Result<()> {
    let shared = SharedState::default();
    {
        let s = shared.clone();
        let h = handle.clone();
        h.spawn(async move {
            if let Ok(addr) = crosstool::start(s).await {
                tracing::info!("cross-tool server on http://{}", addr);
            }
        });
    }

    let native_options = eframe::NativeOptions::default();
    let h = handle.clone();
    let app_shared = shared.clone();
    eframe::run_native(
        "lancer-promptfoo",
        native_options,
        Box::new(
            move |cc| -> Result<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>> {
                Ok(Box::new(App::new(cc, h, app_shared)))
            },
        ),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))
}

struct App {
    handle: tokio::runtime::Handle,
    shared: SharedState,
    ctx: egui::Context,
    config_path: String,
    target: String,
    purpose: String,
    plugins_csv: String,
    strategies_csv: String,
    num_tests: usize,
    offline: bool,
    selected: Option<usize>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, handle: tokio::runtime::Handle, shared: SharedState) -> Self {
        App {
            handle,
            shared,
            ctx: cc.egui_ctx.clone(),
            config_path: String::new(),
            target: "https://example.com/api/chat".to_string(),
            purpose: "Customer-facing AI assistant".to_string(),
            plugins_csv: "owasp:llm,pii:direct,sql-injection,prompt-extraction".to_string(),
            strategies_csv: "base64,jailbreak-templates".to_string(),
            num_tests: 3,
            offline: true,
            selected: None,
        }
    }

    fn start_run(&self) {
        let shared = self.shared.clone();
        let ctx = self.ctx.clone();
        let config_path = self.config_path.clone();
        let target = self.target.clone();
        let purpose = self.purpose.clone();
        let plugins_csv = self.plugins_csv.clone();
        let strategies_csv = self.strategies_csv.clone();
        let num_tests = self.num_tests;
        let offline = self.offline;

        {
            let mut inner = shared.inner.lock().unwrap();
            inner.running = true;
            inner.status = "running…".into();
        }

        self.handle.spawn(async move {
            let config = if !config_path.is_empty() && Path::new(&config_path).exists() {
                PromptfooConfig::load(Path::new(&config_path))
                    .unwrap_or_else(|_| build_config(&target, &purpose, &plugins_csv, &strategies_csv, num_tests))
            } else {
                build_config(&target, &purpose, &plugins_csv, &strategies_csv, num_tests)
            };
            let opts = RunOptions {
                offline,
                cache_enabled: false,
                max_concurrency: 6,
                ..Default::default()
            };
            let result = crate::core::redteam::run(&config, opts).await;
            let mut inner = shared.inner.lock().unwrap();
            inner.running = false;
            match result {
                Ok(report) => {
                    inner.status = format!(
                        "done — {} probes, {} findings",
                        report.vulnerabilities.len(),
                        report.summary.total_findings()
                    );
                    let _ = crate::store::save(&report);
                    inner.report = Some(report);
                }
                Err(e) => inner.status = format!("error: {}", e),
            }
            drop(inner);
            ctx.request_repaint();
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let (running, status, report) = {
            let inner = self.shared.inner.lock().unwrap();
            (inner.running, inner.status.clone(), inner.report.clone())
        };

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Tools", |ui| {
                    if ui.button("Run red team").clicked() {
                        self.start_run();
                        ui.close_menu();
                    }
                });
                ui.menu_button("Help", |ui| {
                    ui.label("lancer-promptfoo");
                    ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
                    let c = crate::core::catalog_counts();
                    ui.label(format!("{} plugins · {} strategies", c.plugins, c.strategies));
                });
            });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(if running { "⏳" } else { "•" });
                ui.label(&status);
                if let Some(r) = &report {
                    ui.separator();
                    ui.label(format!(
                        "C:{} H:{} M:{} L:{}",
                        r.summary.critical, r.summary.high, r.summary.medium, r.summary.low
                    ));
                }
            });
        });

        egui::SidePanel::left("inputs").resizable(true).default_width(280.0).show(ctx, |ui| {
            ui.heading("Target");
            ui.add_space(4.0);
            ui.label("Config path (optional)");
            ui.text_edit_singleline(&mut self.config_path);
            ui.label("Target URL");
            ui.text_edit_singleline(&mut self.target);
            ui.label("Purpose");
            ui.text_edit_multiline(&mut self.purpose);
            ui.label("Plugins (comma-separated)");
            ui.text_edit_multiline(&mut self.plugins_csv);
            ui.label("Strategies (comma-separated)");
            ui.text_edit_singleline(&mut self.strategies_csv);
            ui.horizontal(|ui| {
                ui.label("Tests/plugin");
                ui.add(egui::DragValue::new(&mut self.num_tests));
                if self.num_tests == 0 {
                    self.num_tests = 1;
                }
                if self.num_tests > 50 {
                    self.num_tests = 50;
                }
            });
            ui.checkbox(&mut self.offline, "Offline (echo target, no API calls)");
            ui.add_space(8.0);
            ui.add_enabled_ui(!running, |ui| {
                if ui.button("▶ Run red team").clicked() {
                    self.start_run();
                }
            });
            ui.add_space(8.0);
            ui.separator();
            let c = crate::core::catalog_counts();
            ui.label(format!("Catalog: {} plugins", c.plugins));
            ui.label(format!("{} strategies · {} assertions", c.strategies, c.assertions));
            ui.label(format!("{} providers · {} presets", c.providers, c.presets));
        });

        // Right detail pane.
        egui::SidePanel::right("detail").resizable(true).default_width(360.0).show(ctx, |ui| {
            ui.heading("Detail");
            ui.separator();
            if let (Some(idx), Some(r)) = (self.selected, &report) {
                if let Some(v) = r.vulnerabilities.get(idx) {
                    detail_view(ui, v);
                }
            } else {
                ui.label("Select a finding to see details.");
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Findings");
            ui.separator();
            match &report {
                None => {
                    ui.label("No run yet. Configure a target on the left and click Run.");
                }
                Some(r) => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("findings_grid").striped(true).num_columns(5).show(ui, |ui| {
                            ui.strong("Severity");
                            ui.strong("Plugin");
                            ui.strong("Strategy");
                            ui.strong("Score");
                            ui.strong("Result");
                            ui.end_row();
                            for (i, v) in r.vulnerabilities.iter().enumerate() {
                                let label = format!("{}{}", sev_dot(&v.severity), v.severity.label());
                                if ui.selectable_label(self.selected == Some(i), label).clicked() {
                                    self.selected = Some(i);
                                }
                                ui.label(&v.plugin);
                                ui.label(v.strategy.as_deref().unwrap_or("-"));
                                ui.label(format!("{:.2}", v.score));
                                ui.label(if v.pass { "pass" } else { "FINDING" });
                                ui.end_row();
                            }
                        });
                    });
                }
            }
        });

        if running {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
    }
}

fn detail_view(ui: &mut egui::Ui, v: &Vulnerability) {
    ui.label(egui::RichText::new(format!("[{}] {}", v.severity.label(), v.plugin)).strong());
    ui.label(&v.title);
    ui.separator();
    ui.label(egui::RichText::new("Description").strong());
    ui.label(&v.description);
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Attack prompt").strong());
    egui::ScrollArea::vertical().max_height(160.0).id_source("attack").show(ui, |ui| {
        ui.monospace(&v.attack_prompt);
    });
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Model response").strong());
    egui::ScrollArea::vertical().max_height(200.0).id_source("resp").show(ui, |ui| {
        ui.monospace(&v.model_response);
    });
}

fn sev_dot(s: &Severity) -> String {
    match s {
        Severity::Critical => "🔴 ".into(),
        Severity::High => "🟠 ".into(),
        Severity::Medium => "🟡 ".into(),
        Severity::Low => "🟢 ".into(),
        Severity::Informational => "⚪ ".into(),
    }
}

fn build_config(
    target: &str,
    purpose: &str,
    plugins_csv: &str,
    strategies_csv: &str,
    num_tests: usize,
) -> PromptfooConfig {
    let mut gen = GeneratorOptions::for_web_app(target, purpose);
    gen.num_tests = num_tests;
    gen.plugins = plugins_csv.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    gen.strategies = strategies_csv.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let mut cfg = gen.build();
    // For offline GUI runs the echo target is used regardless; keep the http
    // target in the config for when the user switches offline off.
    if target.is_empty() {
        cfg.targets = vec![ProviderConfig::Id("echo".to_string())];
    }
    cfg
}
