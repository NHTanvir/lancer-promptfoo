//! AUTO-GENERATED framework preset + collection map. Regenerate via the promptfoo-catalogs workflow.
#![allow(clippy::all)]

#[derive(Debug, Clone, Copy)]
pub struct PresetSpec {
    pub id: &'static str,
    pub description: &'static str,
    pub plugins: &'static [&'static str],
}

pub static PRESET_SPECS: &[PresetSpec] = &[
PresetSpec { id: "owasp:llm", description: "Curated OWASP Top 10 for LLM Applications coverage across the most impactful risk categories", plugins: &["prompt-extraction", "system-prompt-override", "indirect-prompt-injection", "harmful:misinformation-disinformation", "overreliance", "hallucination", "pii:direct", "pii:api-db", "excessive-agency", "rbac", "bola", "bfla", "ssrf", "debug-access", "harmful:intellectual-property"] },
PresetSpec { id: "owasp:llm:01", description: "OWASP LLM01 Prompt Injection covering direct and indirect injection vectors", plugins: &["indirect-prompt-injection", "indirect-prompt-injection:rag", "system-prompt-override", "ascii-smuggling", "hijacking", "special-token-injection"] },
PresetSpec { id: "owasp:llm:02", description: "OWASP LLM02 Sensitive Information Disclosure of PII and secrets", plugins: &["pii:direct", "pii:api-db", "pii:session", "pii:social", "cross-session-leak", "harmful:privacy", "prompt-extraction"] },
PresetSpec { id: "owasp:llm:03", description: "OWASP LLM03 Supply Chain risks via poisoned retrieval and dependencies", plugins: &["rag-poisoning", "rag-document-exfiltration", "coding-agent:dependency-injection", "coding-agent:repo-prompt-injection"] },
PresetSpec { id: "owasp:llm:04", description: "OWASP LLM04 Data and Model Poisoning through tampered training or retrieval content", plugins: &["rag-poisoning", "agentic:memory-poisoning", "harmful:misinformation-disinformation", "unverifiable-claims"] },
PresetSpec { id: "owasp:llm:05", description: "OWASP LLM05 Improper Output Handling leading to injection in downstream systems", plugins: &["sql-injection", "shell-injection", "ssrf", "harmful:cybercrime:malicious-code", "coding-agent:generated-vulnerability"] },
PresetSpec { id: "owasp:llm:06", description: "OWASP LLM06 Excessive Agency from over-permissioned tools and actions", plugins: &["excessive-agency", "agentic:tool-misuse", "tool-discovery", "rbac", "bola", "bfla"] },
PresetSpec { id: "owasp:llm:07", description: "OWASP LLM07 System Prompt Leakage exposing instructions and secrets", plugins: &["prompt-extraction", "system-prompt-override", "divergent-repetition"] },
PresetSpec { id: "owasp:llm:08", description: "OWASP LLM08 Vector and Embedding Weaknesses in RAG pipelines", plugins: &["rag-poisoning", "rag-document-exfiltration", "rag-source-attribution", "cross-session-leak"] },
PresetSpec { id: "owasp:llm:09", description: "OWASP LLM09 Misinformation and overreliance on unverified model output", plugins: &["hallucination", "harmful:misinformation-disinformation", "overreliance", "unverifiable-claims", "imitation"] },
PresetSpec { id: "owasp:llm:10", description: "OWASP LLM10 Unbounded Consumption causing denial of service and resource exhaustion", plugins: &["reasoning-dos", "divergent-repetition"] },
PresetSpec { id: "owasp:api", description: "Curated OWASP API Security Top 10 coverage for LLM-backed API endpoints", plugins: &["bola", "bfla", "rbac", "ssrf", "sql-injection", "shell-injection", "pii:api-db", "debug-access", "excessive-agency", "tool-discovery"] },
PresetSpec { id: "owasp:api:01", description: "OWASP API1 Broken Object Level Authorization", plugins: &["bola", "pii:api-db", "pii:session"] },
PresetSpec { id: "owasp:api:02", description: "OWASP API2 Broken Authentication", plugins: &["rbac", "debug-access", "cross-session-leak"] },
PresetSpec { id: "owasp:api:03", description: "OWASP API3 Broken Object Property Level Authorization", plugins: &["bola", "harmful:privacy", "pii:api-db"] },
PresetSpec { id: "owasp:api:04", description: "OWASP API4 Unrestricted Resource Consumption", plugins: &["reasoning-dos", "divergent-repetition"] },
PresetSpec { id: "owasp:api:05", description: "OWASP API5 Broken Function Level Authorization", plugins: &["bfla", "rbac", "excessive-agency"] },
PresetSpec { id: "owasp:api:06", description: "OWASP API6 Unrestricted Access to Sensitive Business Flows", plugins: &["excessive-agency", "agentic:tool-misuse", "ecommerce:price-manipulation"] },
PresetSpec { id: "owasp:api:07", description: "OWASP API7 Server Side Request Forgery", plugins: &["ssrf", "data-exfil"] },
PresetSpec { id: "owasp:api:08", description: "OWASP API8 Security Misconfiguration exposing debug and internal interfaces", plugins: &["debug-access", "shell-injection", "special-token-injection"] },
PresetSpec { id: "owasp:api:09", description: "OWASP API9 Improper Inventory Management of endpoints and tools", plugins: &["tool-discovery", "model-identification", "mcp"] },
PresetSpec { id: "owasp:api:10", description: "OWASP API10 Unsafe Consumption of APIs from untrusted upstream data", plugins: &["indirect-prompt-injection", "rag-poisoning", "ssrf", "data-exfil"] },
PresetSpec { id: "owasp:agentic", description: "OWASP Agentic AI threats covering tool misuse, memory poisoning, and autonomous escalation", plugins: &["excessive-agency", "agentic:tool-misuse", "agentic:memory-poisoning", "tool-discovery", "mcp", "indirect-prompt-injection", "coding-agent:core", "coding-agent:tool-poisoning", "data-exfil", "rbac"] },
PresetSpec { id: "mitre:atlas", description: "Curated MITRE ATLAS adversarial ML tactics across the full attack lifecycle", plugins: &["model-identification", "tool-discovery", "indirect-prompt-injection", "system-prompt-override", "prompt-extraction", "pii:direct", "data-exfil", "rag-poisoning", "agentic:memory-poisoning", "harmful:cybercrime", "reasoning-dos", "excessive-agency"] },
PresetSpec { id: "mitre:atlas:reconnaissance", description: "MITRE ATLAS Reconnaissance probing model identity and capabilities", plugins: &["model-identification", "tool-discovery", "mcp", "prompt-extraction"] },
PresetSpec { id: "mitre:atlas:initial-access", description: "MITRE ATLAS Initial Access via prompt injection and input manipulation", plugins: &["indirect-prompt-injection", "indirect-prompt-injection:rag", "system-prompt-override", "ascii-smuggling", "special-token-injection"] },
PresetSpec { id: "mitre:atlas:persistence", description: "MITRE ATLAS Persistence through memory and retrieval poisoning", plugins: &["agentic:memory-poisoning", "rag-poisoning", "coding-agent:repo-prompt-injection"] },
PresetSpec { id: "mitre:atlas:credential-access", description: "MITRE ATLAS Credential Access targeting secrets and session data", plugins: &["coding-agent:secret-env-read", "coding-agent:secret-file-read", "coding-agent:procfs-credential-read", "pii:session", "cross-session-leak"] },
PresetSpec { id: "mitre:atlas:impact", description: "MITRE ATLAS Impact covering exfiltration, denial of service, and harmful output", plugins: &["data-exfil", "rag-document-exfiltration", "reasoning-dos", "harmful:cybercrime", "excessive-agency"] },
PresetSpec { id: "nist:ai:measure", description: "NIST AI RMF Measure function probing trustworthiness, bias, privacy, and security risks", plugins: &["bias", "bias:age", "bias:disability", "bias:gender", "bias:race", "hallucination", "overreliance", "harmful:privacy", "pii:direct", "harmful:misinformation-disinformation", "excessive-agency", "indirect-prompt-injection"] },
PresetSpec { id: "iso:42001", description: "ISO/IEC 42001 AI management system controls for safety, fairness, and accountability", plugins: &["bias", "harmful:privacy", "pii:direct", "hallucination", "overreliance", "excessive-agency", "harmful:misinformation-disinformation", "harmful:specialized-advice", "harmful:intellectual-property"] },
PresetSpec { id: "gdpr", description: "GDPR personal data protection covering disclosure, profiling, and privacy violations", plugins: &["pii:direct", "pii:api-db", "pii:session", "pii:social", "harmful:privacy", "cross-session-leak", "imitation"] },
PresetSpec { id: "eu:ai-act", description: "EU AI Act prohibited and high-risk practices spanning bias, manipulation, and rights harms", plugins: &["bias", "bias:age", "bias:disability", "bias:gender", "bias:race", "harmful:privacy", "pii:direct", "harmful:misinformation-disinformation", "harmful:hate", "overreliance", "hallucination", "harmful:self-harm"] },
PresetSpec { id: "dod:ai:ethics", description: "DoD AI Ethical Principles ensuring responsible, equitable, traceable, and governable behavior", plugins: &["bias", "bias:race", "bias:gender", "harmful:misinformation-disinformation", "hallucination", "overreliance", "excessive-agency", "harmful:cybercrime", "harmful:indiscriminate-weapons", "harmful:chemical-biological-weapons"] },
PresetSpec { id: "default", description: "Curated default red team suite covering the most common LLM security and safety risks", plugins: &["prompt-extraction", "system-prompt-override", "indirect-prompt-injection", "ascii-smuggling", "hijacking", "pii:direct", "pii:api-db", "harmful:privacy", "excessive-agency", "agentic:tool-misuse", "rbac", "bola", "bfla", "ssrf", "sql-injection", "shell-injection", "debug-access", "hallucination", "overreliance", "harmful:misinformation-disinformation"] },
];

#[derive(Debug, Clone, Copy)]
pub struct CollectionSpec {
    pub id: &'static str,
    pub plugins: &'static [&'static str],
}

pub static COLLECTION_SPECS: &[CollectionSpec] = &[
CollectionSpec { id: "harmful", plugins: &["harmful", "harmful:violent-crime", "harmful:non-violent-crime", "harmful:sex-crime", "harmful:child-exploitation", "harmful:self-harm", "harmful:weapons", "harmful:indiscriminate-weapons", "harmful:chemical-biological-weapons", "harmful:cybercrime", "harmful:illegal-activities", "harmful:unsafe-practices", "harmful:specialized-advice"] },
CollectionSpec { id: "pii", plugins: &["pii:direct", "pii:api-db", "pii:session", "pii:social", "pii:db", "harmful:privacy"] },
CollectionSpec { id: "toxicity", plugins: &["harmful:harassment-bullying", "harmful:hate", "harmful:insults", "harmful:profanity", "harmful:graphic-content", "harmful:sexual-content", "toxic-chat"] },
CollectionSpec { id: "bias", plugins: &["bias", "bias:age", "bias:disability", "bias:gender", "bias:race", "religion"] },
CollectionSpec { id: "medical", plugins: &["medical", "medical:hallucination", "medical:anchoring-bias", "medical:sycophancy", "medical:prioritization-error", "medical:incorrect-knowledge", "medical:off-label-use", "medical:prescription-error", "medical:hipaa", "medical:fda-cyber"] },
CollectionSpec { id: "misinformation", plugins: &["hallucination", "harmful:misinformation-disinformation", "unverifiable-claims", "overreliance", "harmbench:misinformation"] },
CollectionSpec { id: "illegal-activity", plugins: &["harmful:illegal-activities", "harmful:illegal-drugs", "harmful:illegal-drugs:meth", "harmful:violent-crime", "harmful:non-violent-crime", "harmful:sex-crime", "harmful:cybercrime", "harmful:cybercrime:malicious-code", "harmbench:illegal"] },
];
