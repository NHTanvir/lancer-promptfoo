//! Framework preset expansion (OWASP / MITRE ATLAS / NIST / GDPR / EU-AI-Act /
//! DoD / ISO-42001). A preset id expands to a concrete list of plugin ids.

use crate::core::presets_data::PRESET_SPECS;

/// Expand a framework preset id (e.g. `owasp:llm`, `owasp:llm:07`,
/// `mitre:atlas:initial-access`, `nist:ai:measure`) to its plugin ids.
pub fn expand_preset(id: &str) -> Option<Vec<&'static str>> {
    PRESET_SPECS
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.plugins.to_vec())
}

/// True if the id names a known framework preset.
pub fn is_preset(id: &str) -> bool {
    PRESET_SPECS.iter().any(|p| p.id == id)
}

/// All preset ids (sorted) — used by listing commands and the GUI.
pub fn preset_ids() -> Vec<&'static str> {
    let mut ids: Vec<&'static str> = PRESET_SPECS.iter().map(|p| p.id).collect();
    ids.sort_unstable();
    ids
}

pub fn preset_count() -> usize {
    PRESET_SPECS.len()
}
