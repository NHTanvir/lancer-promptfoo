//! Deterministic (no-model) assertion implementations.
//!
//! Each function takes the target output plus the configured value/threshold
//! and returns an [`AssertionOutcome`]. Model-graded assertions (llm-rubric,
//! factuality, g-eval…) are handled by `core::traits::Grader` instead.

use crate::core::types::{AssertionConfig, AssertionOutcome};

fn pass(kind: &str, score: f64, reason: impl Into<String>) -> AssertionOutcome {
    AssertionOutcome {
        kind: kind.to_string(),
        pass: true,
        score,
        reason: reason.into(),
        metric: None,
    }
}

fn fail(kind: &str, score: f64, reason: impl Into<String>) -> AssertionOutcome {
    AssertionOutcome {
        kind: kind.to_string(),
        pass: false,
        score,
        reason: reason.into(),
        metric: None,
    }
}

fn value_str(cfg: &AssertionConfig) -> String {
    match &cfg.value {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

fn value_array(cfg: &AssertionConfig) -> Vec<String> {
    match &cfg.value {
        Some(serde_json::Value::Array(a)) => a
            .iter()
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect(),
        Some(serde_json::Value::String(s)) => s.split(',').map(|p| p.trim().to_string()).collect(),
        _ => Vec::new(),
    }
}

/// Dispatch a deterministic assertion by its `type` string. Returns `None` if
/// the assertion type is model-graded / custom (handled elsewhere).
pub fn evaluate(output: &str, cfg: &AssertionConfig, latency_ms: u64, cost: f64) -> Option<AssertionOutcome> {
    let kind = cfg.kind.as_str();
    let out = match kind {
        "equals" => {
            let v = value_str(cfg);
            if output == v {
                pass(kind, 1.0, "Output equals expected value.")
            } else {
                fail(kind, 0.0, format!("Expected exactly {:?}", v))
            }
        }
        "contains" => {
            let v = value_str(cfg);
            if output.contains(&v) {
                pass(kind, 1.0, format!("Output contains {:?}", v))
            } else {
                fail(kind, 0.0, format!("Output does not contain {:?}", v))
            }
        }
        "icontains" => {
            let v = value_str(cfg).to_lowercase();
            if output.to_lowercase().contains(&v) {
                pass(kind, 1.0, "Output contains expected (case-insensitive).")
            } else {
                fail(kind, 0.0, "Output missing expected (case-insensitive).")
            }
        }
        "starts-with" => {
            let v = value_str(cfg);
            if output.starts_with(&v) {
                pass(kind, 1.0, "Output starts with expected.")
            } else {
                fail(kind, 0.0, "Output does not start with expected.")
            }
        }
        "ends-with" => {
            let v = value_str(cfg);
            if output.ends_with(&v) {
                pass(kind, 1.0, "Output ends with expected.")
            } else {
                fail(kind, 0.0, "Output does not end with expected.")
            }
        }
        "regex" => {
            let v = value_str(cfg);
            match regex::Regex::new(&v) {
                Ok(re) => {
                    if re.is_match(output) {
                        pass(kind, 1.0, format!("Output matches /{}/", v))
                    } else {
                        fail(kind, 0.0, format!("Output does not match /{}/", v))
                    }
                }
                Err(e) => fail(kind, 0.0, format!("Invalid regex: {}", e)),
            }
        }
        "contains-all" => {
            let needles = value_array(cfg);
            let missing: Vec<&String> = needles.iter().filter(|n| !output.contains(n.as_str())).collect();
            let total = needles.len().max(1);
            let score = (total - missing.len()) as f64 / total as f64;
            if missing.is_empty() {
                pass(kind, 1.0, "Output contains all expected substrings.")
            } else {
                fail(kind, score, format!("Missing {} of {} substrings.", missing.len(), total))
            }
        }
        "icontains-all" => {
            let lo = output.to_lowercase();
            let needles = value_array(cfg);
            let missing: Vec<&String> = needles
                .iter()
                .filter(|n| !lo.contains(&n.to_lowercase()))
                .collect();
            let total = needles.len().max(1);
            let score = (total - missing.len()) as f64 / total as f64;
            if missing.is_empty() {
                pass(kind, 1.0, "Output contains all (case-insensitive).")
            } else {
                fail(kind, score, format!("Missing {} of {} (case-insensitive).", missing.len(), total))
            }
        }
        "contains-any" => {
            let needles = value_array(cfg);
            if needles.iter().any(|n| output.contains(n.as_str())) {
                pass(kind, 1.0, "Output contains at least one expected substring.")
            } else {
                fail(kind, 0.0, "Output contains none of the expected substrings.")
            }
        }
        "icontains-any" => {
            let lo = output.to_lowercase();
            let needles = value_array(cfg);
            if needles.iter().any(|n| lo.contains(&n.to_lowercase())) {
                pass(kind, 1.0, "Output contains at least one (case-insensitive).")
            } else {
                fail(kind, 0.0, "Output contains none (case-insensitive).")
            }
        }
        "is-json" => match serde_json::from_str::<serde_json::Value>(output.trim()) {
            Ok(_) => pass(kind, 1.0, "Output is valid JSON."),
            Err(e) => fail(kind, 0.0, format!("Output is not valid JSON: {}", e)),
        },
        "contains-json" => {
            if extract_any_json(output).is_some() {
                pass(kind, 1.0, "Output embeds a JSON value.")
            } else {
                fail(kind, 0.0, "No JSON value found in output.")
            }
        }
        "is-html" => {
            let t = output.trim();
            if t.starts_with('<') && t.ends_with('>') && t.contains("</") {
                pass(kind, 1.0, "Output looks like HTML.")
            } else {
                fail(kind, 0.0, "Output is not well-formed HTML.")
            }
        }
        "contains-html" => {
            if output.contains('<') && output.contains("</") && output.contains('>') {
                pass(kind, 1.0, "Output contains HTML tags.")
            } else {
                fail(kind, 0.0, "Output contains no HTML tags.")
            }
        }
        "is-refusal" => {
            if looks_like_refusal(output) {
                pass(kind, 1.0, "Output is a refusal.")
            } else {
                fail(kind, 0.0, "Output is not a refusal.")
            }
        }
        "latency" => {
            let threshold = cfg.threshold.unwrap_or(0.0) as u64;
            if latency_ms <= threshold {
                pass(kind, 1.0, format!("Latency {}ms <= {}ms", latency_ms, threshold))
            } else {
                fail(kind, 0.0, format!("Latency {}ms > {}ms", latency_ms, threshold))
            }
        }
        "cost" => {
            let threshold = cfg.threshold.unwrap_or(0.0);
            if cost <= threshold {
                pass(kind, 1.0, format!("Cost {:.6} <= {:.6}", cost, threshold))
            } else {
                fail(kind, 0.0, format!("Cost {:.6} > {:.6}", cost, threshold))
            }
        }
        "levenshtein" => {
            let v = value_str(cfg);
            let dist = levenshtein(output, &v);
            let threshold = cfg.threshold.unwrap_or(5.0) as usize;
            if dist <= threshold {
                pass(kind, 1.0, format!("Levenshtein distance {} <= {}", dist, threshold))
            } else {
                fail(kind, 0.0, format!("Levenshtein distance {} > {}", dist, threshold))
            }
        }
        "rouge-n" | "rouge" => {
            let v = value_str(cfg);
            let score = rouge_n(output, &v, 1);
            scored(kind, score, cfg.threshold.unwrap_or(0.5), "ROUGE-1 recall")
        }
        "bleu" | "gleu" => {
            let v = value_str(cfg);
            let score = bleu(output, &v);
            scored(kind, score, cfg.threshold.unwrap_or(0.5), "BLEU (unigram)")
        }
        "f1" => {
            let v = value_str(cfg);
            let score = token_f1(output, &v);
            scored(kind, score, cfg.threshold.unwrap_or(0.5), "Token F1")
        }
        "meteor" => {
            let v = value_str(cfg);
            // METEOR approximation: harmonic mean weighted toward recall.
            let p = token_precision(output, &v);
            let r = token_recall(output, &v);
            let score = if p + r > 0.0 {
                (10.0 * p * r) / (r + 9.0 * p)
            } else {
                0.0
            };
            scored(kind, score, cfg.threshold.unwrap_or(0.5), "METEOR (approx)")
        }
        // Anything else is model-graded / custom / trajectory — not deterministic.
        _ => return None,
    };
    Some(out)
}

fn scored(kind: &str, score: f64, threshold: f64, label: &str) -> AssertionOutcome {
    if score >= threshold {
        pass(kind, score, format!("{} {:.3} >= {:.3}", label, score, threshold))
    } else {
        fail(kind, score, format!("{} {:.3} < {:.3}", label, score, threshold))
    }
}

pub fn looks_like_refusal(output: &str) -> bool {
    let r = output.to_lowercase();
    const MARKERS: &[&str] = &[
        "i can't",
        "i cannot",
        "i'm not able",
        "i am not able",
        "i won't",
        "i will not",
        "i'm sorry",
        "i am sorry",
        "cannot help",
        "can't help",
        "not appropriate",
        "against my",
        "unable to assist",
        "i must decline",
        "i'm unable",
    ];
    MARKERS.iter().any(|m| r.contains(m))
}

fn extract_any_json(text: &str) -> Option<serde_json::Value> {
    if let Some(start) = text.find(['{', '[']) {
        let opener = text.as_bytes()[start] as char;
        let closer = if opener == '{' { '}' } else { ']' };
        if let Some(end) = text.rfind(closer) {
            if end > start {
                if let Ok(v) = serde_json::from_str(&text[start..=end]) {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// Classic Wagner–Fischer Levenshtein edit distance over chars.
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

fn tokens(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

/// ROUGE-N recall (fraction of reference n-grams found in the candidate).
pub fn rouge_n(candidate: &str, reference: &str, n: usize) -> f64 {
    let cand = ngrams(&tokens(candidate), n);
    let refr = ngrams(&tokens(reference), n);
    if refr.is_empty() {
        return 0.0;
    }
    let hits = refr.iter().filter(|g| cand.contains(*g)).count();
    hits as f64 / refr.len() as f64
}

fn ngrams(tokens: &[String], n: usize) -> Vec<String> {
    if n == 0 || tokens.len() < n {
        return tokens.to_vec();
    }
    tokens.windows(n).map(|w| w.join(" ")).collect()
}

/// Unigram BLEU-style precision with a brevity penalty.
pub fn bleu(candidate: &str, reference: &str) -> f64 {
    let cand = tokens(candidate);
    let refr = tokens(reference);
    if cand.is_empty() {
        return 0.0;
    }
    let hits = cand.iter().filter(|t| refr.contains(*t)).count();
    let precision = hits as f64 / cand.len() as f64;
    let bp = if cand.len() >= refr.len() {
        1.0
    } else {
        (1.0 - refr.len() as f64 / cand.len() as f64).exp()
    };
    precision * bp
}

fn token_precision(candidate: &str, reference: &str) -> f64 {
    let cand = tokens(candidate);
    let refr = tokens(reference);
    if cand.is_empty() {
        return 0.0;
    }
    cand.iter().filter(|t| refr.contains(*t)).count() as f64 / cand.len() as f64
}

fn token_recall(candidate: &str, reference: &str) -> f64 {
    let cand = tokens(candidate);
    let refr = tokens(reference);
    if refr.is_empty() {
        return 0.0;
    }
    refr.iter().filter(|t| cand.contains(*t)).count() as f64 / refr.len() as f64
}

pub fn token_f1(candidate: &str, reference: &str) -> f64 {
    let p = token_precision(candidate, reference);
    let r = token_recall(candidate, reference);
    if p + r == 0.0 {
        0.0
    } else {
        2.0 * p * r / (p + r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(kind: &str, value: serde_json::Value) -> AssertionConfig {
        AssertionConfig {
            kind: kind.to_string(),
            value: Some(value),
            ..Default::default()
        }
    }

    #[test]
    fn contains_and_equals() {
        let o = evaluate("hello world", &cfg("contains", "world".into()), 0, 0.0).unwrap();
        assert!(o.pass);
        let o = evaluate("hello", &cfg("equals", "hello".into()), 0, 0.0).unwrap();
        assert!(o.pass);
        let o = evaluate("hello", &cfg("equals", "bye".into()), 0, 0.0).unwrap();
        assert!(!o.pass);
    }

    #[test]
    fn is_json_and_regex() {
        assert!(evaluate(r#"{"a":1}"#, &cfg("is-json", serde_json::Value::Null), 0, 0.0).unwrap().pass);
        assert!(evaluate("abc123", &cfg("regex", r"\d+".into()), 0, 0.0).unwrap().pass);
    }

    #[test]
    fn levenshtein_distance() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("", "abc"), 3);
    }

    #[test]
    fn model_graded_returns_none() {
        assert!(evaluate("x", &cfg("llm-rubric", "be nice".into()), 0, 0.0).is_none());
    }
}
