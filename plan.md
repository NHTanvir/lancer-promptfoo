# RESEARCH DOSSIER: Promptfoo

> Part of the Lancer ecosystem. A Rust wrapper + extension layer around Promptfoo — the dominant open-source LLM red-teaming framework. Build with Claude Code.
> Upstream: github.com/promptfoo/promptfoo (TypeScript, 21.6k+ stars, weekly releases). Used internally by OpenAI and Anthropic per their README.

## Feature Parity Mandate

**`lancer-promptfoo` MUST ship with 100% feature parity to the upstream `promptfoo` tool — every flag, every subcommand, every output format, every probe / detector / module / payload / mode — plus a native Rust GUI on top.** This is not a "subset clone" — this is a full reimplementation in Rust with a GUI wrapper.

### Parity contract

- **Every CLI flag** documented in upstream `promptfoo --help` (and the long-form man page / docs) must work in `lancer-promptfoo --cli`.
- **Every output format** the upstream tool emits (text, JSON, XML, CSV, SARIF, custom formats) must round-trip through `lancer-promptfoo` byte-equivalent where the format allows.
- **Every probe / detector / module / category / payload class / built-in script** must be ported. None silently dropped.
- **Every supported target type / input format / protocol / file format / language** must work.
- **Every configuration option** (config files, env vars, registry / settings entries) must be honoured.
- **Companion binaries / sub-tools** in the upstream suite (if any) must each get their own binary in this workspace.

### GUI on top

Every feature reachable via CLI is also reachable from the GUI (see `## GUI (Lancer-standard pattern)` below). The GUI is not a subset — it is an alternate frontend over the same `core` library that the CLI uses. The `core` crate is GUI-agnostic.

### Verification

- The `## Granular feature catalog` section below enumerates everything that must be supported.
- Reference: `tests/upstream_parity/` will pin upstream flag combinations + canned outputs as parity tests. CI fails if any flag is unsupported or output diverges.
- A `lancer-promptfoo --self-test` flag runs the full parity matrix against a bundled reference target and prints pass/fail per upstream feature group.

### What is allowed to diverge

- **Performance** — Rust-native, async-first; we are allowed to be faster.
- **Memory footprint** — we are allowed to be smaller.
- **GUI** — entirely new (no upstream GUI to copy except where the upstream ships its own GUI like Zenmap / Wireshark UI / Ghidra GUI / jadx-gui — in which case our GUI must cover every panel the upstream GUI has, plus the Lancer-standard layout from `## GUI` below).
- **AI augmentation** — net-new (see `## AI augmentation ideas`).

### What is NOT allowed to diverge

- Any flag silently dropped, renamed, or behavior-changed compared to upstream.
- Any output format / record schema diverging in field names, types, or semantics.
- Any built-in script / detector / module not ported (mark explicitly with `[N/A — see note]` if genuinely impossible in Rust, e.g. NSE Lua scripts requiring features not in `mlua`).

## What it actually does

Promptfoo is the dominant open-source LLM **red-teaming + evaluation** framework. It does two related jobs from one config file (`promptfooconfig.yaml`):

1. **Eval mode** — run prompts/agents/RAGs against a battery of test cases with deterministic and LLM-as-judge assertions, side-by-side compare GPT/Claude/Gemini/DeepSeek/Ollama, generate diffs, gate PRs.
2. **Red-team mode** — auto-generate thousands of application-specific attack probes via 157+ plugins, deliver them through 25+ strategies (encoding, jailbreak, crescendo, GOAT), grade responses with an LLM judge, produce OWASP/NIST/MITRE/EU-AI-Act compliance reports.

The killer feature is the **plugin × strategy matrix**: 157 plugins × 25 strategies = ~3,900 conceptual attack-vector combinations, generated on demand per target. A user runs `npx promptfoo@latest redteam setup`, edits `promptfooconfig.yaml`, runs `promptfoo redteam run`, and gets a HackerOne-grade vuln report.

Latest as of May 2026: **v0.x.x** (410 releases since v0.1; `code-scan-action 0.1.6` released 2026-05-21). The pace is ~weekly on `main`.

## Rust port viability — HONEST verdict

**Don't full-port. Wrap + extend.**

Reasoning:
- Promptfoo is ~500k lines of TypeScript with a React/Vite UI, Express + Socket.IO server, SQLite via drizzle-orm, Commander CLI, and a Docusaurus docs site. Porting that to Rust is a 6-12 month full-time effort that buys you **nothing the user actually needs** — Promptfoo is not CPU-bound; >99% of wall time is LLM API latency.
- The 157 plugins are mostly **prompt templates + grading rubrics in TypeScript files**. Reproducing them all in Rust = grueling translation work with no functional improvement.
- The plugin marketplace effect: when promptfoo ships `coding-agent:steganographic-exfil` next month, you'd be perpetually re-porting.

**Recommendation for Lancer:** Keep promptfoo as a subprocess. Build a Rust integration crate `lancer-llm-redteam` that:
1. Spawns `promptfoo redteam run` via tokio process API.
2. Parses its JSON output (`promptfoo eval --output report.json`).
3. Translates findings into Lancer's internal vulnerability schema.
4. Adds Lancer-specific things promptfoo doesn't have: Burp-style request replay/tamper of the attack prompts, integration with Lancer's OAST (out-of-band) collaborator, and your own Rust-native LLM client (e.g. via `async-openai` or `litellm-rust`) for any custom strategies you want to add.

The one piece worth writing in Rust: a **fast streaming HTTP custom-provider shim** that accepts promptfoo's HTTP attack POSTs, replays them through Lancer's proxy chain (so you get full request/response capture in the same Burp-like UI as everything else). That's ~500 LoC of `axum` + `reqwest` and unlocks "promptfoo attacks visible in Lancer's HTTP history" — a real product win.

## Internal architecture (with code refs)

**Top-level monorepo layout** (Node ^20.20, TypeScript 6.x, tsdown + tsc build):

```
promptfoo/
├── src/                          # core CLI + library
│   ├── main.ts                   # Commander.js CLI entry (evalCommand, redteamBaseCommand)
│   ├── index.ts                  # library entry (exports `evaluate()`)
│   ├── evaluator.ts              # core eval loop: renders prompts → calls providers → runs assertions
│   ├── assertions/               # deterministic + model-graded assertions
│   ├── providers/                # 30+ LLM provider abstractions
│   │   └── providerRegistry.ts   # provider lookup
│   ├── redteam/                  # adversarial test generation
│   │   ├── plugins/              # 157 attack plugins
│   │   ├── strategies/           # 25 delivery strategies
│   │   ├── providers/            # red-team-specific providers (iterativeImage, etc.)
│   │   ├── graders/              # LLM-as-judge graders
│   │   ├── grading/
│   │   ├── extraction/           # data exfil utilities
│   │   ├── audio/                # multimodal audio strategy
│   │   ├── commands/             # `promptfoo redteam ...` CLI
│   │   ├── constants/
│   │   ├── shared/
│   │   ├── types/
│   │   ├── graders.ts
│   │   ├── metrics.ts
│   │   ├── riskScoring.ts
│   │   ├── inputVariables.ts
│   │   ├── mcpMaterialization.ts # MCP integration
│   │   ├── mcpTargetProvider.ts
│   │   ├── remoteGeneration.ts   # offload attack-generation to promptfoo cloud
│   │   └── remoteMaterialization.ts
│   ├── server/                   # Express 5 + Socket.IO backend
│   ├── app/src/                  # React 19 + Vite 8 + Zustand + Radix UI frontend
│   ├── models/eval.ts            # SQLite persistence via drizzle-orm + better-sqlite3
│   ├── util/config/              # YAML loader + Zod validators
│   └── types/index.ts            # TS types for config
├── plugins/                      # external plugin marketplace dir
├── examples/                     # ~80 example configs
├── docs/                         # Docusaurus site
├── helm/chart/promptfoo/         # Kubernetes deploy
└── code-scan-action/             # GitHub Action for PR scanning
```

**Eval engine flow** (`src/evaluator.ts`):

```
loadConfig() → parseYaml → Zod-validate
  → for each test:
      renderPrompt(template, vars)
      for each provider:
          response = provider.callApi(prompt)
          for each assertion:
              score = assertion.evaluate(response)
          persist(evalResult) to SQLite
  → write JSON + HTML report
  → optional: push to web UI via Socket.IO
```

**Red-team flow** (`src/redteam/commands/run.ts`):

```
loadRedteamConfig() → expand plugins (e.g. owasp:llm → 30 plugin IDs)
  → for each plugin:
      generateAttacks(plugin, target.purpose, numTests)
                    ↑ calls remoteGeneration (promptfoo cloud) OR local LLM
  → for each strategy:
      strategy.transform(attacks)   # e.g. crescendo: split into multi-turn
  → evaluate(target, transformedAttacks, graders)
  → riskScoring → severity buckets
  → write report (HTML + JSON + CSV)
```

**Provider abstraction** — every provider implements:
```ts
interface ApiProvider {
  id(): string;
  callApi(prompt: string, ctx: CallApiContextParams, opts?: CallApiOptionsParams): Promise<ProviderResponse>;
}
```

Where `ProviderResponse = { output: string, tokenUsage?, cached?, error?, raw?, metadata? }`.

Known providers: OpenAI, Anthropic, Bedrock, Azure, Vertex/Gemini, Ollama, DeepSeek, HTTP (custom REST), WebSocket, Python script, JavaScript script, Browser, GitHub Models, Hugging Face, Mistral, Cohere, Groq, Cloudflare AI Gateway, Replicate, Together, vLLM, Watsonx, plus agent-SDK wrappers (LangChain, LlamaIndex, OpenAI Assistants).

**Plugin pattern** — each plugin is a TypeScript module exporting:
```ts
interface RedteamPlugin {
  id: string;
  generateTests(numTests: number, purpose: string, vars: Record<string,string>): Promise<TestCase[]>;
  grader: Grader;   // LLM rubric used to score response
}
```

The grader is itself a prompt template fed to a judge model (default: `openai:gpt-4.1` or `anthropic:claude-sonnet-4.6`).

**Strategy pattern** — each strategy is a TS module exporting:
```ts
interface Strategy {
  id: string;
  transform(tests: TestCase[]): Promise<TestCase[]>; // mutate/expand
  isMultiTurn?: boolean;
}
```

Multi-turn strategies (`crescendo`, `goat`, `mischievous-user`, `jailbreak:hydra`) return tests with a `conversationOrchestrator` callback that runs N turns and decides when to stop.

## Complete feature catalog

### Red-team plugins — 157 total across 6 categories

**Brand & Reputation (14)**: competitors, excessive-agency, financial:counterfactual, financial:defamation, financial:hallucination, financial:sycophancy, goal-misalignment, hallucination, imitation, harmful:misinformation-disinformation, off-topic, overreliance, politics, unverifiable-claims.

**Compliance & Legal (48)**: harmful:chemical-biological-weapons, harmful:copyright-violations, coppa, ferpa, harmful:cybercrime, harmful:illegal-activities, harmful:illegal-drugs, harmful:indiscriminate-weapons, harmful:intellectual-property, harmful:non-violent-crime, harmful:sex-crime, harmful:specialized-advice, harmful:unsafe-practices, harmful:violent-crime, contracts, rag-document-exfiltration, plus vertical packs (`realestate:*` for Fair Housing Act, `telecom:*` for FCC/CALEA/TCPA, `pharmacy:*` for DEA/dosage, `insurance:*` for ADA/PHI/GLBA, `financial:*` for SOX/MNPI/FIEA, `ecommerce:*` for PCI-DSS, `medical:*` for FDA cyber).

**Dataset (11)**: aegis (NVIDIA), beavertails, cyberseceval (Meta), donotanswer, harmbench, pliny (L1B3RT4S), toxic-chat, unsafebench, vlguard, vlsu, xstest.

**Security & Access Control (51)**: ascii-smuggling, cca (Context Compliance Attacks), the full `coding-agent:*` pack (automation-poisoning, core, delayed-ci-exfil, generated-vulnerability, network-egress-bypass, procfs-credential-read, repo-prompt-injection, sandbox-read-escape, sandbox-write-escape, secret-env-read, secret-file-read, steganographic-exfil, terminal-output-injection, verifier-sabotage — 13 sub-plugins targeting Cursor/Devin/Claude-Code-style agents), cross-session-leak, data-exfil, debug-access, pii:direct, pii:api-db, pii:session, pii:social, divergent-repetition, hijacking, indirect-prompt-injection, harmful:cybercrime:malicious-code, ssrf, agentic:memory-poisoning, mcp (MCP attacks), model-identification, harmful:privacy, bfla, bola, prompt-extraction, rag-poisoning, rag-source-attribution, rbac, reasoning-dos, shell-injection, special-token-injection, sql-injection, system-prompt-override, tool-discovery.

**Trust & Safety (38)**: bias:age, bias:disability, bias:gender, bias:race, harmful:child-exploitation, harmful:graphic-content, harmful:harassment-bullying, harmful:hate, harmful:weapons:ied, harmful:insults, harmful:profanity, harmful:radicalization, harmful:self-harm, harmful:sexual-content, religion, wordplay, full `teen-safety:*` pack, full `medical:*` truth pack.

**Custom (2)**: `policy` (user provides natural-language policy, plugin generates probes) and `intent` (user provides exact strings).

### Strategies — 25 total in 3 cost tiers

**Recommended** (jailbreak:meta, jailbreak:hydra) — adaptive learned-taxonomy strategies. 70-90% ASR gain.

**Static / low-cost** (12 deterministic encoders): base64, hex, rot13, homoglyph, jailbreak-templates (DAN, Skeleton Key), leetspeak, camelCase, audio, image, video, morse, pig-latin, emoji (UTF-8 in variation selectors). 0-30% ASR gain.

**Dynamic single-turn / iterative** (9): jailbreak (LLM-as-judge iterative refinement, PAIR-style), jailbreak:composite, best-of-n, gcg, tree (Tree of Attacks with Pruning, TAP), citation, likert, math-prompt, authoritative-markup-injection. 40-80% ASR gain.

**Multi-turn** (3): crescendo (Microsoft research), goat (Meta's GOAT), mischievous-user. 10-90% ASR.

**Specialized**: indirect-web-pwn, retry.

**Composition**: `layer` chains strategies (e.g. `base64 → rot13`). Custom strategies can be JS file (`file://strategy.js`) or natural-language instructions.

### Framework presets

| Preset | Coverage |
|---|---|
| `default` | Promptfoo's curated starter set (~25 plugins) |
| `owasp:llm` | OWASP LLM Top 10 v2025 (LLM01-LLM10) |
| `owasp:api` | OWASP API Security Top 10 |
| `owasp:agentic` | OWASP Top 10 for AI Agents (newer 2026 list) |
| `mitre:atlas` | MITRE ATLAS framework |
| `nist:ai:measure` | NIST AI RMF measure subcategories |
| `iso:42001` | ISO/IEC 42001 AI management |
| `gdpr` | GDPR articles (e.g. `gdpr:art5`) |
| `eu:ai-act` | EU AI Act articles |

### Eval assertion types

Deterministic: `equals`, `contains`, `starts-with`, `ends-with`, `regex`, `contains-all`, `contains-any`, `is-json`, `is-valid-openai-tools-call`, `latency`, `cost`, `levenshtein`, `rouge-n`, `bleu`, `f1`, `perplexity`.

Model-graded: `llm-rubric`, `factuality`, `answer-relevance`, `context-recall`, `context-precision`, `context-relevance`, `context-faithfulness`, `model-graded-closedqa`, `model-graded-factuality`, `pi-scorer` (prompt-injection grader), `classifier`.

Custom: `javascript:file://...`, `python:file://...`, `webhook`.

### CI/CD integrations

- GitHub Action (`promptfoo/code-scan-action`) — comments on PRs with attack success diffs vs base branch.
- GitLab CI, CircleCI, Azure DevOps — standard `npm install -g promptfoo && promptfoo redteam run --no-progress-bar`.
- Slack/Webhook notifications on assertion fails.

## Engineering hard parts

1. **YAML → runtime config compilation.** Zod schemas validate ~30 top-level keys, polymorphic plugin entries, and provider polymorphism. Invalid YAML produces unhelpful errors; promptfoo has a deep error-recovery layer.
2. **Attack generation cost.** Naively, 157 plugins × 25 strategies × 5 tests each = ~20k generation LLM calls. Promptfoo offloads generation to its remote-generation endpoint by default.
3. **Concurrency + rate limits.** Per-provider semaphore with adaptive backoff on 429s. Default concurrency 4; configurable per-provider.
4. **Caching + dedup.** SQLite-backed cache keyed by `(provider, prompt, options-hash)`. Important because grading runs a judge model on every response — dedup saves 60%+ of cost on rerun.
5. **Multi-turn orchestration.** Crescendo/GOAT have to maintain conversation state, decide turn-by-turn whether to escalate or backtrack, and stop when the judge says "vulnerability triggered" — implemented as async generators with a `state` object passed between turns.
6. **Scoring rubric design.** Each plugin's grader prompt has to be specific enough to not false-positive on benign refusals ("I can't help with that") but generic enough to catch novel evasions.
7. **Plugin composition vs explosion.** `owasp:llm` expands to ~30 plugin IDs; each can be overridden individually.

## Library mapping for Rust port (if you ignore my "don't port" advice)

| Promptfoo dep | Rust equivalent |
|---|---|
| Commander.js (CLI) | `clap` v4 |
| Express + Socket.IO | `axum` + `tokio-tungstenite` |
| better-sqlite3 + drizzle | `rusqlite` or `sqlx` |
| Zod schema validation | `serde` + `validator` or `garde` |
| YAML parsing | `serde_yaml` |
| OpenAI/Anthropic clients | `async-openai`, `litellm-rust` |
| Multi-provider abstraction | `litellm-rust` (avivsinai) or `rig` |
| Jinja-ish templating | `minijinja` |
| Playwright (for browser strategy) | `playwright-rust` or shell out to `chromiumoxide` |
| React frontend | Tauri + same React (UI doesn't need to be Rust) |
| Tokenization | `tiktoken-rs` |
| HTTP custom provider | `reqwest` + `tower` |
| Concurrency / semaphore | `tokio::sync::Semaphore` |
| Tracing | `tracing` + `tracing-subscriber` |

**Plugin system in Rust**:
```rust
#[async_trait]
pub trait RedteamPlugin: Send + Sync {
    fn id(&self) -> &str;
    async fn generate_tests(
        &self,
        n: usize,
        purpose: &str,
        vars: &HashMap<String, String>,
    ) -> Result<Vec<TestCase>>;
    fn grader(&self) -> Arc<dyn Grader>;
}

pub struct PluginRegistry {
    plugins: HashMap<String, Arc<dyn RedteamPlugin>>,
}
```

## 2024-2026 state

- **v0.x growth**: 410 GitHub releases; weekly minor releases on `main`.
- **2025 — OWASP LLM Top 10 v2025 integration**: full LLM01-LLM10 v2025 mapping added; older 2023 list deprecated.
- **2025 — MITRE ATLAS preset added**.
- **2026 — `owasp:agentic` preset** for OWASP Top 10 for AI Agents (handles MCP, memory poisoning, automation chains).
- **2026 — `coding-agent:*` plugin pack** (13 plugins targeting Cursor/Devin/Claude-Code/Copilot Workspace) — sandbox escapes, secret env reads, steganographic exfil, verifier sabotage. Big in 2026 bug-bounty programs.
- **2026 — vertical plugin packs**: realestate/telecom/medical/pharmacy/insurance/financial/ecommerce.
- **2026 — Hydra (multi-turn) + Meta Agent** added as top-tier adaptive strategies.
- **2026 — `code-scan-action` v0.1.6** (May 2026) — GitHub Action for PR-time scanning.

### Research note — 2026-05-30 deep-research pass (vs upstream)

Deep-research against `github.com/promptfoo/promptfoo` (21.7k stars, 412 releases, weekly cadence), `promptfoo.dev/docs`, and 2026 news cycle. **Major findings + gaps that MUST be backfilled into `lancer-promptfoo`:**

**1. Corporate context (NEW, 2026-03):**
- **OpenAI acquired Promptfoo, March 9, 2026.** Tech is being integrated into **OpenAI Frontier** (launched 2026-02-05). Promptfoo team led by Ian Webster + Michael D'Angelo. Tool stays MIT-licensed, open-source, independent CLI. Trusted by **25% of Fortune 500**. Lancer parity contract is unaffected — but expect upstream cadence to potentially slow or skew toward OpenAI-Frontier integration features.
- Sibling OpenAI launch same day: **Codex Security** (agent code vuln scanning) — overlaps with promptfoo's `coding-agent:*` plugin pack.

**2. CLI subcommands previously NOT enumerated in this plan — MUST be added to parity matrix:**
- `promptfoo optimize` — iterative prompt improvement against a provider.
- `promptfoo code-scans run` — repository-level LLM vulnerability scan (new, 2026).
- `promptfoo scan-model` — ML model file security risk assessment (think: pickle exec, malicious weights).
- `promptfoo mcp` — start a **Model Context Protocol server** so promptfoo itself is callable from Claude/Cursor/Continue.
- `promptfoo validate` — config file + provider connectivity precheck.
- `promptfoo logs` — debug / error log access.
- `promptfoo delete` — remove specific evaluations.
- `promptfoo retry` — re-run only failed cases from a previous eval.
- `promptfoo show <id>` — inspect a resource by ID.
- `promptfoo auth login|logout|whoami|teams|can-create-targets` — cloud account + org-team switching + permission probing.
- `promptfoo config` — manage user settings (e.g. email for cloud).
- `promptfoo feedback` — send improvement suggestions upstream.
- `promptfoo debug` — diagnostic output.
- `promptfoo redteam plugins` — list available plugin IDs (new subcommand surfaced 2026).

**3. `eval` flags previously underspecified — MUST be in `lancer-promptfoo eval`:**
- Filtering: `--filter-first-n`, `--filter-range`, `--filter-pattern`, `--filter-metadata`, `--filter-prompts`, `--filter-providers`.
- Output: `-o/--output` extended to **CSV, JSON, YAML, HTML, XML, JUnit** (JUnit = CI gating output, was missing).
- Execution: `--max-concurrency`, `--delay`, `--repeat`, `--watch` (live reload — was only mentioned in passing).
- Advanced: `--resume`, `--retry-errors`, `--suggest-prompts`, `--tag key=value` (CI/CD context tagging without editing source config).
- Sharing: `--share` / `--no-share` (cloud upload toggle).

**4. Providers — previously listed 30+; full 2026 catalog is broader. Backfill these IDs:**
- **Hosted-LLM aggregators / gateways:** `litellm`, `vercel-ai-gateway`, `helicone`, `cloudflare-ai-gateway`, `cloudflare-workers-ai`, `openrouter`, `github-models`.
- **High-perf inference:** `groq`, `cerebras`, `together`, `fireworks`, `hyperbolic`, `nvidia-nim`, `sambanova`, `lambda-labs`.
- **Hosted majors not yet listed:** `xai` (Grok), `perplexity`, `ai21`, `databricks`, `snowflake-cortex`, `meta-llama`, `watsonx`, `voyage`, `jina`, `adaline`.
- **Local / self-hosted:** `ollama`, `localai`, `llama-cpp`, `vllm`, `docker-model-runner`, `llamafile`.
- **Integration shims:** `http`, `https`, `websocket`, `python` (script file), `javascript` (script file), `mcp` (Model Context Protocol), `browser` (Playwright/Chromium), `webhook`.
- Lancer parity contract: each provider ID becomes a Rust factory in `core::providers::registry`; provider-specific config keys mirror upstream YAML schema exactly.

**5. Assertions — previously enumerated ~25; 2026 catalog adds these:**
- Deterministic: `icontains`, `contains-any` / `icontains-any`, `contains-all` / `icontains-all`, `is-html` / `contains-html`, `is-refusal`, `ruby` (was missing — alongside `javascript` / `python`), `gleu`, `meteor`, `perplexity-score`, `is-valid-function-call`, `guardrails` (NEW — runs Llama Guard / OpenAI moderation locally), **trajectory metrics** (agent tool-use validation — `trajectory:goal-success`, tool-call sequence checks).
- Model-graded: `g-eval` (chain-of-thought eval framework — was listed but spec is richer in 2026), `conversation-relevance` (multi-turn consistency), `pi` (dedicated Lakera-style scoring model), `max-score` (highest aggregate selection across N candidates), `model-graded-closedqa` (OpenAI evals method).

**6. Red-team strategies — 2026 inventory is 30+ (this plan listed 25). Backfill / re-key:**
- Note **strategy ID renaming** vs older docs: `emoji` → `emoji-smuggling`, `camelCase` → `camelcase`, `morse` → `morse-code`, `jailbreak:meta` → `meta`, `jailbreak:hydra` → `hydra`, `composite` → `composite-jailbreaks`. Lancer must accept BOTH legacy + 2026 IDs (alias map) so old YAMLs still work.
- Add explicit Regression tier: `retry` (replays historical failures from prior eval DB).
- Add Custom tier: `custom` (user JS), `custom-strategy` (natural-language multi-turn description compiled by the meta-agent), `layer` (chain N strategies sequentially, e.g. `base64 → rot13 → leetspeak`).

**7. Plugin presets — backfill these previously-missing presets:**
- `dod:ai:ethics` — US DoD AI Ethics principles preset (NEW for federal customers).
- `nist:ai:measure:<N.N>` — granular sub-measure addressability (not just the top-level `nist:ai:measure`).
- `owasp:llm:01` … `owasp:llm:10` — addressable per-item (not only the bundle).
- `owasp:api:01` … `owasp:api:10` — same per-item addressability.
- `mitre:atlas:reconnaissance|initial-access|persistence|credential-access|impact` — per-tactic addressability.
- Plugin **collections** (semantic bundles): `harmful`, `pii`, `toxicity`, `bias`, `medical`, `misinformation`, `illegal-activity` — these expand to multiple plugin IDs at load time.

**8. OWASP LLM Top 10 v2025 — confirm the two NEW top-level entries are covered by Lancer:**
- **LLM08: Vector & Embedding Weaknesses** (NEW 2025) — targets RAG vector DB pollution, embedding inversion, retrieval poisoning. Backed by `rag-poisoning`, `rag-document-exfiltration`, `rag-source-attribution`.
- **LLM07: System Prompt Leakage** (NEW 2025) — backed by `prompt-extraction`, `system-prompt-override`, `divergent-repetition`.
- **OWASP Top 10 for Agentic Applications** announced at **Black Hat Europe 2025** + OWASP Agentic Security Summit; mapped to `owasp:agentic` preset; covers MCP, memory poisoning, automation chains, tool discovery, excessive agency. Already present — flag as production-ready.

**9. CI / cloud / share — gaps to close in Lancer:**
- **Cloud share URLs** (`promptfoo share` + `--share`) — uploads eval results to `app.promptfoo.dev` and returns a public read-only URL. Lancer must (a) implement an outbound HTTP uploader, (b) honor `--no-share` to keep results local, (c) support a private/self-hosted share endpoint via env var `PROMPTFOO_SHARING_ENDPOINT`.
- **Cloud auth** — `auth login` opens a browser PKCE flow; tokens stored in OS keyring (`keyring` crate on Rust side).
- **Org / team switching** — `auth teams` switches the active org for billing + RBAC. Lancer's CLI must mirror the same JSON token format so existing `~/.promptfoo/auth.json` works unchanged.
- **GitHub Action** — `promptfoo/code-scan-action@v0.1.6` (May 2026). Lancer must publish a sibling `lancer-promptfoo/code-scan-action` or document a drop-in wrapper that calls `lancer-promptfoo redteam run --baseline base.json --output sarif`.
- **SARIF output** — confirm `--output report.sarif` is supported (GitHub Code Scanning ingestion). Was listed in parity contract but verify implementation.
- **JUnit output** — `--output report.junit.xml` for Jenkins / GitLab / Azure DevOps test-result gating.
- **Slack / webhook notifications** — `notifications:` config block; fire on eval-fail or red-team CRITICAL/HIGH.
- **Watch mode** (`--watch`) — file watcher on `promptfooconfig.yaml`, prompts dir, vars dir; debounced re-run with HMR-style result delta in the viewer.

**10. Viewer UI / web frontend — parity items:**
- `promptfoo view` launches the React/Vite SPA at `http://localhost:15500`. Lancer's `egui` GUI is a separate frontend — but for users invoking `lancer-promptfoo view --legacy`, we must EITHER (a) embed a static-built copy of the upstream React app and serve it via `axum` and `tower-http`, OR (b) provide a parity-feature note saying the native GUI replaces it.
- WebSocket live-update channel (`/ws` via Socket.IO in upstream, `tokio-tungstenite` in Lancer).
- Eval comparison view (side-by-side diff across providers).
- Red-team report HTML (severity dashboard, plugin × strategy matrix heatmap, per-finding repro card).

**11. MCP server mode — NEW capability:**
- `promptfoo mcp` exposes promptfoo itself as an MCP server so Claude Code / Cursor / Continue can call `promptfoo.eval`, `promptfoo.redteam_run`, `promptfoo.assert` as tools. Lancer must implement `lancer-promptfoo mcp` against the same MCP spec (JSON-RPC 2.0 over stdio + SSE).

**12. Auth bypass / red-team plumbing — env vars to honor:**
- `PROMPTFOO_DISABLE_REDTEAM_REMOTE_GENERATION=1` (force local generation only — no calls to promptfoo cloud).
- `HF_TOKEN` (HuggingFace dataset plugins: aegis, beavertails, harmbench, donotanswer).
- `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GOOGLE_API_KEY`, `AWS_*`, `AZURE_*`, `GROQ_API_KEY`, `MISTRAL_API_KEY`, `COHERE_API_KEY`, `TOGETHER_API_KEY`, `OPENROUTER_API_KEY`, `XAI_API_KEY`, `DEEPSEEK_API_KEY`, etc.
- `PROMPTFOO_CACHE_PATH`, `PROMPTFOO_LOG_DIR`, `PROMPTFOO_CONFIG_DIR`.

**Net delta:** ~12 CLI subcommands, ~18 eval flags, ~20 providers, ~15 assertions, ~5 strategies, ~6 framework-preset families that were under-specified or missing from this plan. All must be added to the `## Granular feature catalog` and `tests/upstream_parity/` matrix. None of the existing content is removed — this is purely additive.

**Vs competitors (2026 verdict):**

| Tool | Stars | Probes | Strength | Best fit |
|---|---|---|---|---|
| **Promptfoo** | 21.6k | 157 plugins × 25 strategies | YAML configs, CI/CD native, vertical packs, compliance presets | Engineering teams, regression testing, bug-bounty against productized chatbots |
| Garak (NVIDIA) | ~5k | ~100 probes, 37+ modules | "Nmap for LLMs" | CI regression on model deploys |
| PyRIT (Microsoft) | ~3k | Multi-modal, multi-turn (Crescendo, TAP) | Programmable orchestration | Security researchers doing quarterly deep dives |
| DeepTeam | ~2k | 40+ vulns, OWASP-aligned | DeepEval-integrated | Teams already using DeepEval |

Promptfoo wins on **breadth + compliance reporting + CI ergonomics**.

## Bug-bounty workflow

```bash
# 1. Init config for a target chatbot
mkdir target-acme && cd target-acme
npx promptfoo@latest redteam init --target https://acme.com/api/chat
```

```yaml
targets:
  - id: https
    label: acme-customer-bot
    config:
      url: https://acme.com/api/chat
      method: POST
      headers:
        Authorization: 'Bearer {{ env.ACME_TOKEN }}'
        Content-Type: application/json
      body:
        message: '{{prompt}}'
        conversation_id: '{{conversationId}}'
      transformResponse: 'json.reply'

redteam:
  purpose: 'Customer-service chatbot for Acme telecom...'
  numTests: 30
  plugins:
    - owasp:llm
    - telecom:account-takeover
    - pii:direct
    - sql-injection
    - ssrf
    - bola
    - excessive-agency
    - indirect-prompt-injection
    - prompt-extraction
  strategies:
    - jailbreak:meta
    - jailbreak:hydra
    - crescendo
    - base64
    - tree
  provider: anthropic:claude-sonnet-4.6
```

```bash
promptfoo redteam run --no-progress-bar
promptfoo redteam report   # opens HTML dashboard
promptfoo eval --output findings.json
```

Triage by severity (CRITICAL/HIGH only for bounty), reproduce manually in Lancer Repeater, write report.

**Productive 2026 LLM bounty programs to target:** Anthropic, OpenAI, Microsoft Copilot, Google Gemini, Cohere/Perplexity/Mistral, HackerOne/Bugcrowd AI categories.

### Research note — 2026-05-31 (v0.121.6 → v0.121.13 + code-scan-action 0.1.6/0.1.7 deltas)

Direct walk of `github.com/promptfoo/promptfoo/releases` last 20 releases. Additions to capture:

- **New LLM model providers / aliases (May 2026).** `claude-opus-4-8`, `gpt-5.5`, `gemini-3.5-flash`, `minimax`, `novita`, `nvidia-nim`, `orca-router`. Our provider registry must accept all by name (provider IDs as enum + string-to-enum lookup).
- **New aggregation providers.** **`mlflow-ai-gateway`** (target = `mlflow:gateway/<route>` with MLflow Deployments token), **`n8n` webhook target** (target = `n8n:webhook/<workflow-id>` so promptfoo can probe n8n workflows directly), **`atlas-cloud`** (target = `atlas:<model>`), **`hyperbolic-image`** / **`hyperbolic-audio`** (image gen + audio diffusion). Our `lancer-promptfoo` provider matrix needs these 5 new targets.
- **Video generation providers.** **OpenAI Sora** and **Google Veo** added as text-to-video targets. Assertion semantics for video are bytes-+-metadata, not text — judge is multimodal (Gemini 2.5 Vision or Claude 4.6 Vision). Our provider trait gains a `Modality::Video` variant; assertion `video:contains-frame` / `video:no-deepfake` / `video:audio-aligned`.
- **xAI Voice Agent** as audio-conversation target. Audio in, audio out. Our `Modality::Audio` variant + assertion `audio:transcribe-match`.
- **Google AI Studio embeddings** as new embedding provider (separate from Vertex). For `embedding-similarity` assertion.
- **Red-team plugins (May 2026).** **DOCX input wrapper generation** — given a target that accepts file uploads, wraps every attack in a `.docx` payload so file-parsing endpoints are exercised. **HarmBench category filtering** — `plugins: [harmbench]` now supports `harmbench:misinformation`, `harmbench:cybercrime`, `harmbench:bioweapons`, `harmbench:chembio` sub-categories. **Coding-agent collection expansion** — `plugins: [coding-agent]` now expands to a curated set of coding-specific attacks (path-traversal in generated code, command-injection in shell-suggestions, dependency-confusion attacks on package suggestions, license-violation generation).
- **Red-team strategies (May 2026).** **`hydra`** — adaptive multi-turn that dynamically branches based on target response patterns (Tree-of-Attacks-Pruning-with-history). **`meta`** (renamed from `jailbreak:meta`) — uses the OWASP LLM-jailbreak taxonomy to learn which sub-strategy works on this target and prefers that. **Context purpose overrides** — `strategies: [{ id: "crescendo", config: { contextPurpose: "..." }}]` lets you override the conversation goal per-strategy. **Remote input materialization** — strategy variants can fetch input bodies from a remote URL at runtime (for confidential corpora kept off-disk).
- **Assertion types (May 2026).** **`trajectory:tool-args-match`** — matches tool call args from an agent trajectory against expected glob patterns with optional default-tolerance value. **XML indexed-path** — `is-xpath: //user[@id="123"]/name[text()]` now supports indexed paths like `//user[@id="123"]/orders[2]/total`. **Vercel AI SDK tool-call spans** — recognizes Vercel AI SDK trace format and asserts on tool spans.
- **MCP additions (May 2026).** **Response transforms for tool outputs** — `mcp.transforms: { '<tool>': '$.result.data | jq ".[0]"' }` lets you post-process tool results before passing to assertion. **Anthropic MCP tool result execution** — Anthropic-format MCP servers now have their tool results re-executed inside the eval (verifies the side-effect, not just the response). **Bedrock converse MCP support** — Bedrock Converse API targets can now expose MCP tools.
- **CLI additions (May 2026).** **`promptfoo optimize`** — prompt-optimization workflow that runs the eval, picks the worst-performing prompts, asks an LLM to rewrite, re-runs eval, iterates until plateau. **Team selector with search** — `promptfoo share --team <name>` now interactive-searches the team list. **ModelAudit scanner selection** — `--model-audit-scanners=protectai,modelscan` chooses which model scanners run during ModelAudit phase. **Eval filter range** — `--filter-range '#10-50'` picks test IDs 10 through 50. **OpenAI Evals import** — `promptfoo import-openai-evals <id>` pulls an OpenAI Evals dashboard test set into a local config.
- **Evaluation mode additions.** **Runtime variables in eval context** — `vars: { now: "{{date.now}}", session: "{{uuid}}" }` resolved at eval-runtime, not config-load. **Named metric columns for CSV** — `--csv-metrics latency,cost,accuracy` exports specified metrics as labelled columns. **JUnit XML reporting** — `--output report.junit.xml` for CI integration. **HTML detail drawer** — clicking a result row in the HTML dashboard opens a side-drawer with full prompt + response + trace. **`author` option in eval config + metadata from hooks** — `author: jane@acme.com` and lifecycle hooks can attach arbitrary metadata to runs.
- **CI/CD additions.** **`promptfoo/code-scan-action` v0.1.7** outputs SARIF for GitHub Code Scanning. Structured fork-PR skip output for security boundary. Minimum-severity alias (`min-severity: high` ↔ `minimum_severity: high`). Test descriptions emitted into HTML output for traceability.
- **Runtime tags + custom-policy config (May 2026).** **`promptfoo redteam run --tag bb-2026 --tag staging`** attaches runtime tags to a redteam run. **Custom-policy config** — `policies: ./my-policies.yaml` lets you point at an org-wide policy file (denylist + tone rules + brand constraints) used by the LLM judge to score policy compliance per-finding.

Sources: [promptfoo releases](https://github.com/promptfoo/promptfoo/releases), [OWASP LLM Top 10](https://www.promptfoo.dev/docs/red-team/owasp-llm-top-10/), [OWASP Top 10 for Agentic Applications](https://www.promptfoo.dev/docs/red-team/owasp-agentic-ai/), [Promptfoo Red Team Configuration](https://www.promptfoo.dev/docs/red-team/configuration/), [Promptfoo 2026 Complete Guide](https://qaskills.sh/blog/promptfoo-complete-guide-2026).

## AI augmentation (using Claude inside Promptfoo)

**1. Claude as target.** Trivial:
```yaml
targets:
  - id: anthropic:claude-sonnet-4.6
    config:
      apiKey: '{{ env.ANTHROPIC_API_KEY }}'
      max_tokens: 2048
      temperature: 0
```

**2. Claude as attack generator AND judge** (recommended — Claude is stronger than GPT for adversarial reasoning):
```yaml
redteam:
  provider:
    id: anthropic:claude-opus-4.7
    config:
      max_tokens: 4096
      temperature: 0.8     # high temp for creative attacks
defaultTest:
  options:
    provider: anthropic:claude-sonnet-4.6   # judge model
```

**Prompt caching:** When using Claude as judge across thousands of tests, enable prompt caching to cut judge cost ~70%:
```yaml
defaultTest:
  options:
    provider:
      id: anthropic:claude-sonnet-4.6
      config:
        extra_headers:
          anthropic-beta: 'prompt-caching-2024-07-31'
```

## Validation corpus (for clone correctness)

1. **OWASP LLM Top 10 v2025 official examples** — genai.owasp.org/llm-top-10/
2. **HarmBench** — 510 harmful behaviors with known-passing attacks.
3. **BeaverTails** — 30k+ Q-A pairs labeled across 14 harm categories.
4. **DoNotAnswer** — 939 prompts a well-aligned model should refuse.
5. **L1B3RT4S / Pliny corpus** — community jailbreak library.
6. **Anthropic public jailbreak benchmarks** — "Many-shot Jailbreaking" paper test set.
7. **PortSwigger Web Security Academy AI labs** — eight LLM labs covering OWASP LLM Top 10.
8. **AI CTF challenges** — AI-village's "Generative Red Team" CTF from DEF CON 31/32.

## Implementation phases — HONEST 12-week plan (for the "wrap + augment" approach)

| Week | Deliverable |
|---|---|
| 1 | `lancer-llm-redteam` crate skeleton. Tokio subprocess wrapper around `promptfoo redteam run`. Parse JSON output into Lancer's `Vulnerability` struct. |
| 2 | HTTP custom-provider shim in `axum` — receives promptfoo attack POSTs, routes through Lancer's proxy chain. |
| 3 | Lancer UI tab: "LLM Red Team" — config wizard that writes `promptfooconfig.yaml`, run button, live progress via Socket.IO bridge. |
| 4 | OAST integration — when a strategy is `ssrf` or `data-exfil`, auto-inject Lancer's OAST collaborator domain into attack payloads, correlate hits. |
| 5 | Rust-native LLM client layer using `litellm-rust` — for custom Lancer-only strategies you can't express in promptfoo YAML. |
| 6 | Custom Lancer plugin: `lancer:web-app-context` — pulls system prompt from a JS bundle, injects it into attack generation context. |
| 7 | Findings dedup + correlation with Lancer's existing HTTP scanner findings. |
| 8 | Replay UI — every promptfoo-generated attack is replayable + editable in Lancer Repeater. |
| 9 | Bounty report templates — auto-generate Markdown reports from severity:HIGH/CRITICAL findings. |
| 10 | CI mode — `lancer redteam --baseline base.json --fail-on critical` for GitHub Actions. |
| 11 | Validation against PortSwigger LLM labs, HarmBench subset; tune grader thresholds. |
| 12 | Docs, examples, blog post: "Lancer for LLM red-teaming." |

**Total honest LoC estimate: ~6-8k Rust + minor TS PR upstream to promptfoo for clean JSON output.** Full port would be 80-120k LoC and 6-12 months — don't.

---

## GUI (Lancer-standard pattern)

`lancer-promptfoo` ships as a **native GUI application** by default. The CLI surface listed in `## Granular feature catalog` is still available behind a `--cli` flag (or via the `lancer-promptfoo-cli` companion binary), but the **plain `lancer-promptfoo` invocation opens a native desktop window**, matching every other tool in the Lancer ecosystem. No terminal-only mode.

### Framework

Pure-Rust GUI stack — no JavaScript, no Node, single static binary:

| Crate | Role |
|-------|------|
| `eframe` | Cross-platform window + event loop wrapper around `egui` |
| `egui` | Immediate-mode UI widgets (panels, tables, forms, menus, drag-and-drop) |
| `egui_extras` | Sortable tables, image loaders, syntax highlighting, date pickers |
| `egui_plot` | Charts, graphs, timelines (histograms, activity meters, progress) |
| `rfd` | Native file/folder pickers (open file, save dialog) |
| `directories` | Per-OS config dir (`~/.config/lancer-promptfoo/`, `%APPDATA%\lancer-promptfoo\`, etc.) |
| `image` | Screenshot / preview rendering inside egui |
| `arboard` | Clipboard read/write for "copy result" actions |
| `muda` | Native menu bar (Linux / macOS / Windows) |
| `egui-notify` | Toast notifications |

Optional dark/light theme persisted to `lancer-promptfoo-config.json` via `serde_json`.

### Window layout

Three-pane layout matching the Lancer mothership GUI:

```
┌─────────────────────────────────────────────────────────────────────┐
│  File   Tools   Settings   Help                              [≡]    │
├──────────┬────────────────────────────────────────┬─────────────────┤
│          │                                        │                 │
│  Inputs  │           Main view                    │  Detail pane    │
│  / jobs  │  (table of results / source tree /     │  (full record / │
│  / scope │   findings list / packet list / etc.)  │   bytes / src / │
│          │                                        │   remediation)  │
│          │                                        │                 │
├──────────┴────────────────────────────────────────┴─────────────────┤
│  Status: idle           Items: 0          [Open]  [Run]  [Export]   │
└─────────────────────────────────────────────────────────────────────┘
```

- **Left sidebar (200-300px)** — inputs / queued jobs / scope / target list. Drag-and-drop accepted.
- **Center pane (flexible)** — primary result view appropriate to the tool (table for findings, tree for hierarchical data, code editor for source).
- **Right pane (300-400px, collapsible)** — full detail of the currently-selected item.
- **Top menu bar** — File (Open / Save / Export / Recent / Quit), Tools (per-tool actions), Settings (theme, fonts, integrations), Help (About / Docs).
- **Bottom status bar** — current activity, item counts, primary action buttons.

### Cross-tool integration with Lancer + the other 29 clones

Every Lancer-clone GUI exposes a small local HTTP server on a port auto-allocated at launch (`127.0.0.1:<port>`, written to `%TEMP%/lancer-promptfoo.port`):

- The main Lancer GUI discovers running tools by scanning `%TEMP%/lancer-*.port` and lists them in its sidebar.
- Right-clicking any item in any tool can "Send to <other Lancer tool>" — the receiving tool fetches the payload via its own HTTP endpoint.
- A future "Lancer Hub" tray shell can launch / focus any of the 30 GUIs from one menu.
- HTTP endpoints exposed:
  - `GET  /v1/state` — current jobs / progress
  - `POST /v1/import` — accept a target / file / record from another Lancer tool
  - `GET  /v1/export?format=json|csv|sarif|markdown` — pull the current result set in a portable format

### Async work + responsiveness

- Long-running scans run on a Tokio task pool; the GUI thread only renders.
- Progress propagates via `crossbeam::channel` from worker → UI.
- A `Stop` button cancels in-flight work via a `CancellationToken`.
- All file I/O / network I/O / subprocess work goes through async channels — the egui thread never blocks.

### Build output

The existing `build.bat` / `build-windows.bat` / `build-linux.sh` already produce a single binary. With `eframe` linked, the same binary is the GUI app.

- Windows: `build\lancer-promptfoo.exe` (double-click opens the GUI window).
- Linux: `build/lancer-promptfoo` (also installable as a `.desktop` entry via `build-linux.sh --install`).
- macOS: `build/lancer-promptfoo` (bundled into `lancer-promptfoo.app` via `cargo-bundle` in the `build-mac.sh` companion).

For CI / headless / scripting, pass `--cli` to fall through to the CLI surface from `## Granular feature catalog`.

## Build Setup (Lancer-standard pattern)

This clone follows the same build pattern as the main Lancer project — single `.exe` output via `build.bat` on Windows, `.so/elf` via `build-linux.sh` on Linux.

### Project layout

```
lancer-promptfoo/
├── Cargo.toml              # single-crate Rust project
├── Cargo.lock              # committed (binary, not library)
├── src/
│   └── main.rs             # entry point (scaffold)
├── build.bat               # Windows build → build/lancer-promptfoo.exe
├── build-windows.bat       # Windows packaged dist build → dist/lancer-promptfoo-windows-x86_64/
├── build-linux.sh          # Linux build → build/lancer-promptfoo
├── .gitignore              # plan.md (local-only) + target/ + build/ + dist/
├── README.md               # public-facing (pushed to GitHub)
└── plan.md                 # this file (gitignored — local blueprint, never pushed)
```

### Build commands

**Windows (double-click `build.bat`):**
```
build.bat
```
Produces `build\lancer-promptfoo.exe`. Pauses at end so the window stays open.

**Windows (packaged dist):**
```
build-windows.bat
```
Produces `dist\lancer-promptfoo-windows-x86_64\bin\lancer-promptfoo.exe` plus LICENSE/README — ready to zip.

**Linux / macOS:**
```bash
./build-linux.sh
```
Produces `build/lancer-promptfoo`.

### Requirements
- Rust toolchain (https://rustup.rs)
- Windows: Visual C++ Build Tools 2019/2022 (needed by some crates)

### CI

The build pattern is intentionally identical across Lancer and all 19 clones, so a single Jenkins/GitHub Actions matrix can build everything by calling `build-windows.bat` or `build-linux.sh` per repo.
