//! `core` — the GUI-agnostic engine of lancer-promptfoo.
//!
//! Everything the CLI and the GUI need lives here: the config schema, the
//! provider/plugin/strategy/assertion registries, the eval + red-team engines,
//! the reporters, the cache, and the self-test. Nothing in `core` depends on a
//! frontend crate (parity mandate: "the `core` crate is GUI-agnostic").

pub mod assertions;
pub mod cache;
pub mod config;
pub mod engine;
pub mod env;
pub mod eval;
pub mod plugins;
pub mod presets;
pub mod presets_data;
pub mod promptfoo_json;
pub mod providers;
pub mod redteam;
pub mod report;
pub mod selftest;
pub mod strategies;
pub mod traits;
pub mod types;

/// Aggregate catalog counts — surfaced by `--self-test`, `validate`, the GUI
/// "about" panel, and the parity tests.
pub struct CatalogCounts {
    pub providers: usize,
    pub plugins: usize,
    pub strategies: usize,
    pub assertions: usize,
    pub presets: usize,
    pub collections: usize,
}

pub fn catalog_counts() -> CatalogCounts {
    CatalogCounts {
        providers: providers::ProviderRegistry::new().len(),
        plugins: plugins::PluginRegistry::new().len(),
        strategies: strategies::StrategyRegistry::new().len(),
        assertions: assertions::AssertionRegistry::new().len(),
        presets: presets::preset_count(),
        collections: presets_data::COLLECTION_SPECS.len(),
    }
}
