//! Semantic plugin collections (`harmful`, `pii`, `toxicity`, `bias`,
//! `medical`, `misinformation`, `illegal-activity`). A collection id expands to
//! several concrete plugin ids at config-load time.

use crate::core::presets_data::COLLECTION_SPECS;

pub fn expand_collection(id: &str) -> Option<Vec<&'static str>> {
    COLLECTION_SPECS
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.plugins.to_vec())
}

pub fn is_collection(id: &str) -> bool {
    COLLECTION_SPECS.iter().any(|c| c.id == id)
}

pub fn collection_ids() -> Vec<&'static str> {
    let mut ids: Vec<&'static str> = COLLECTION_SPECS.iter().map(|c| c.id).collect();
    ids.sort_unstable();
    ids
}
