//! AUTO-GENERATED strategy catalog (32+) with legacy aliases. Regenerate via the promptfoo-catalogs workflow.
#![allow(clippy::all)]
use crate::core::traits::{StrategySpec, StrategyTier, StrategyKind};

pub static STRATEGY_SPECS: &[StrategySpec] = &[
StrategySpec { id: "meta", tier: StrategyTier::Recommended, kind: StrategyKind::Iterative, multi_turn: false, description: "OWASP jailbreak-taxonomy adaptive strategy that auto-selects effective attacks.", asr_gain: "70-90%", aliases: &["jailbreak:meta"] },
StrategySpec { id: "hydra", tier: StrategyTier::Recommended, kind: StrategyKind::MultiTurn, multi_turn: true, description: "Multi-turn adaptive orchestration that branches across several conversational paths.", asr_gain: "70-90%", aliases: &["jailbreak:hydra"] },
StrategySpec { id: "base64", tier: StrategyTier::Static, kind: StrategyKind::Encoder, multi_turn: false, description: "Base64-encode the attack payload.", asr_gain: "0-30%", aliases: &[] },
StrategySpec { id: "hex", tier: StrategyTier::Static, kind: StrategyKind::Encoder, multi_turn: false, description: "Hex-encode the attack payload to bypass content filters.", asr_gain: "0-30%", aliases: &[] },
StrategySpec { id: "rot13", tier: StrategyTier::Static, kind: StrategyKind::Encoder, multi_turn: false, description: "Apply the ROT13 letter-rotation cipher to the attack payload.", asr_gain: "0-30%", aliases: &[] },
StrategySpec { id: "homoglyph", tier: StrategyTier::Static, kind: StrategyKind::Encoder, multi_turn: false, description: "Substitute lookalike Unicode homoglyph characters to evade keyword filters.", asr_gain: "20-50%", aliases: &[] },
StrategySpec { id: "leetspeak", tier: StrategyTier::Static, kind: StrategyKind::Encoder, multi_turn: false, description: "Rewrite the attack payload in leetspeak character substitutions.", asr_gain: "0-30%", aliases: &[] },
StrategySpec { id: "camelcase", tier: StrategyTier::Static, kind: StrategyKind::Encoder, multi_turn: false, description: "Concatenate words in camelCase to obscure the attack payload.", asr_gain: "0-30%", aliases: &["camelCase"] },
StrategySpec { id: "morse-code", tier: StrategyTier::Static, kind: StrategyKind::Encoder, multi_turn: false, description: "Encode the attack payload as Morse code.", asr_gain: "0-30%", aliases: &["morse"] },
StrategySpec { id: "pig-latin", tier: StrategyTier::Static, kind: StrategyKind::Encoder, multi_turn: false, description: "Transform the attack payload into Pig Latin.", asr_gain: "0-30%", aliases: &[] },
StrategySpec { id: "emoji-smuggling", tier: StrategyTier::Static, kind: StrategyKind::Encoder, multi_turn: false, description: "Hide the attack payload inside emoji variation-selector sequences.", asr_gain: "20-50%", aliases: &["emoji"] },
StrategySpec { id: "ascii-smuggling", tier: StrategyTier::Static, kind: StrategyKind::Encoder, multi_turn: false, description: "Smuggle the attack payload using invisible ASCII tag Unicode characters.", asr_gain: "20-50%", aliases: &[] },
StrategySpec { id: "binary", tier: StrategyTier::Static, kind: StrategyKind::Encoder, multi_turn: false, description: "Encode the attack payload as binary digits.", asr_gain: "0-30%", aliases: &[] },
StrategySpec { id: "url-encode", tier: StrategyTier::Static, kind: StrategyKind::Encoder, multi_turn: false, description: "Percent-encode the attack payload as a URL-encoded string.", asr_gain: "0-30%", aliases: &[] },
StrategySpec { id: "reverse", tier: StrategyTier::Static, kind: StrategyKind::Encoder, multi_turn: false, description: "Reverse the characters of the attack payload.", asr_gain: "0-30%", aliases: &[] },
StrategySpec { id: "jailbreak-templates", tier: StrategyTier::Static, kind: StrategyKind::PassThrough, multi_turn: false, description: "Wrap the attack payload in known static jailbreak prompt templates.", asr_gain: "30-60%", aliases: &[] },
StrategySpec { id: "audio", tier: StrategyTier::Static, kind: StrategyKind::Remote, multi_turn: false, description: "Render the attack payload as synthesized audio for multimodal targets.", asr_gain: "20-50%", aliases: &[] },
StrategySpec { id: "image", tier: StrategyTier::Static, kind: StrategyKind::Remote, multi_turn: false, description: "Render the attack payload as text inside an image for multimodal targets.", asr_gain: "20-50%", aliases: &[] },
StrategySpec { id: "video", tier: StrategyTier::Static, kind: StrategyKind::Remote, multi_turn: false, description: "Render the attack payload as a video for multimodal targets.", asr_gain: "20-50%", aliases: &[] },
StrategySpec { id: "docx", tier: StrategyTier::Static, kind: StrategyKind::Remote, multi_turn: false, description: "Embed the attack payload inside a DOCX document attachment.", asr_gain: "20-50%", aliases: &[] },
StrategySpec { id: "jailbreak", tier: StrategyTier::Dynamic, kind: StrategyKind::Iterative, multi_turn: false, description: "Single-turn iterative PAIR-style refinement that adapts the prompt to the target.", asr_gain: "60-80%", aliases: &[] },
StrategySpec { id: "composite-jailbreaks", tier: StrategyTier::Dynamic, kind: StrategyKind::Iterative, multi_turn: false, description: "Chain multiple known jailbreak techniques into a single composite attack.", asr_gain: "60-80%", aliases: &["composite"] },
StrategySpec { id: "best-of-n", tier: StrategyTier::Dynamic, kind: StrategyKind::Iterative, multi_turn: false, description: "Sample many augmented prompt variants and keep the best-performing one.", asr_gain: "40-70%", aliases: &[] },
StrategySpec { id: "gcg", tier: StrategyTier::Dynamic, kind: StrategyKind::Iterative, multi_turn: false, description: "Append a greedy-coordinate-gradient adversarial suffix to the prompt.", asr_gain: "40-70%", aliases: &[] },
StrategySpec { id: "tree", tier: StrategyTier::Dynamic, kind: StrategyKind::Iterative, multi_turn: false, description: "Tree-of-attacks-with-pruning search over candidate prompt branches.", asr_gain: "60-80%", aliases: &[] },
StrategySpec { id: "citation", tier: StrategyTier::Dynamic, kind: StrategyKind::Iterative, multi_turn: false, description: "Frame the attack payload with fabricated academic citations to add false authority.", asr_gain: "30-60%", aliases: &[] },
StrategySpec { id: "likert", tier: StrategyTier::Dynamic, kind: StrategyKind::Iterative, multi_turn: false, description: "Use a Likert-scale grading frame to coax graded harmful completions.", asr_gain: "30-60%", aliases: &[] },
StrategySpec { id: "math-prompt", tier: StrategyTier::Dynamic, kind: StrategyKind::Iterative, multi_turn: false, description: "Disguise the attack payload as a symbolic mathematics or set-theory problem.", asr_gain: "40-70%", aliases: &[] },
StrategySpec { id: "authoritative-markup-injection", tier: StrategyTier::Dynamic, kind: StrategyKind::Iterative, multi_turn: false, description: "Inject authoritative-looking markup framing to override the system instructions.", asr_gain: "30-60%", aliases: &[] },
StrategySpec { id: "crescendo", tier: StrategyTier::MultiTurn, kind: StrategyKind::MultiTurn, multi_turn: true, description: "Gradually escalate a multi-turn conversation toward the harmful objective.", asr_gain: "70-90%", aliases: &[] },
StrategySpec { id: "goat", tier: StrategyTier::MultiTurn, kind: StrategyKind::MultiTurn, multi_turn: true, description: "Generative offensive agent tester that adapts attacks across conversation turns.", asr_gain: "70-90%", aliases: &[] },
StrategySpec { id: "mischievous-user", tier: StrategyTier::MultiTurn, kind: StrategyKind::MultiTurn, multi_turn: true, description: "Simulate a persistently mischievous user probing the target over many turns.", asr_gain: "60-80%", aliases: &[] },
StrategySpec { id: "indirect-web-pwn", tier: StrategyTier::Specialized, kind: StrategyKind::Remote, multi_turn: false, description: "Plant indirect prompt-injection payloads in web content the agent retrieves.", asr_gain: "40-70%", aliases: &[] },
StrategySpec { id: "retry", tier: StrategyTier::Regression, kind: StrategyKind::Regression, multi_turn: false, description: "Replay previously discovered failing test cases to catch regressions.", asr_gain: "varies", aliases: &[] },
StrategySpec { id: "layer", tier: StrategyTier::Composition, kind: StrategyKind::Composite, multi_turn: false, description: "Compose several strategies in sequence by layering their transforms.", asr_gain: "varies", aliases: &[] },
StrategySpec { id: "custom", tier: StrategyTier::Custom, kind: StrategyKind::Custom, multi_turn: false, description: "User-supplied custom strategy loaded from a JavaScript file.", asr_gain: "varies", aliases: &[] },
StrategySpec { id: "custom-strategy", tier: StrategyTier::Custom, kind: StrategyKind::Custom, multi_turn: false, description: "User-supplied custom strategy described in natural language.", asr_gain: "varies", aliases: &[] },
];

pub fn all_strategy_specs() -> Vec<StrategySpec> { STRATEGY_SPECS.to_vec() }
pub fn spec_ref(id: &str) -> Option<&'static StrategySpec> {
    STRATEGY_SPECS.iter().find(|s| s.id == id || s.aliases.contains(&id))
}
