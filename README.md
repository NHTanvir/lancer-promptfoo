# lancer-promptfoo

A **Rust reimplementation + native GUI** of [promptfoo](https://promptfoo.dev) — the
LLM red-team & evaluation framework — built for the Lancer 2026 AI-era bug-bounty toolkit.

It is not a thin wrapper: it ships a native Rust engine (eval + red-team), a 157+ plugin
catalog, 30+ delivery strategies, 45+ assertion types, ~60 provider integrations, all the
upstream output formats, an MCP server mode, and a desktop GUI — plus an optional subprocess
bridge to the upstream `promptfoo` CLI when you want the exact upstream engine.

> Default invocation opens the **native GUI**. Pass a subcommand (or `--cli`) for the CLI,
> or use the headless `lancer-promptfoo-cli` binary.

## Features

- **Eval mode** — prompts × tests × providers with deterministic + model-graded assertions.
- **Red-team mode** — 157+ plugins × 30+ strategies (the plugin × strategy matrix), graded
  by an LLM-as-judge, with OWASP/MITRE/NIST/GDPR/EU-AI-Act/DoD compliance presets.
- **Providers (~60)** — OpenAI, Anthropic, Bedrock, Vertex/Gemini, Azure, Mistral, Cohere,
  xAI, DeepSeek, Groq, Together, Fireworks, OpenRouter, LiteLLM, Ollama, vLLM, plus 2026
  additions (mlflow, n8n, atlas-cloud, Sora/Veo video, hyperbolic image/audio, xAI voice…).
- **Strategies** — native encoders (base64, hex, rot13, leetspeak, morse, homoglyph,
  emoji/ascii-smuggling, …), framing wrappers (jailbreak-templates, citation, math-prompt,
  likert), and the dynamic/multi-turn tiers (jailbreak, tree, crescendo, goat, hydra, meta).
  Legacy ids (`jailbreak:meta`, `camelCase`, `morse`, `emoji`, `composite`) still resolve.
- **Assertions (45+)** — equals/contains/regex/is-json/levenshtein/rouge/bleu/f1 (native) +
  llm-rubric/factuality/g-eval/guardrails/trajectory (model-graded).
- **Output formats** — JSON, YAML, CSV, HTML dashboard, XML, **SARIF** (GitHub Code Scanning),
  **JUnit** (CI gating), Markdown bounty report, plain text.
- **MCP server** — `lancer-promptfoo mcp` exposes `eval` / `redteam_run` / `assert` as tools.
- **HTTP capture shim** — `lancer-promptfoo serve` proxies attacks through Lancer's HTTP
  history for Burp-style replay.
- **Cloud** — `auth` / `share` (honors `PROMPTFOO_SHARING_ENDPOINT` and `--no-share`).
- **Native GUI** — three-pane egui desktop app with a cross-tool HTTP server for the Lancer
  ecosystem (`/v1/state`, `/v1/import`, `/v1/export`).

## Usage

```bash
# Scaffold a config for a target
lancer-promptfoo redteam init --target https://acme.com/api/chat --preset web-app

# Run the red team (offline echo target for a dry run)
lancer-promptfoo redteam run --config promptfooconfig.yaml --offline -o report.html -o report.sarif

# CI gating
lancer-promptfoo redteam run --baseline base.json --fail-on critical -o report.junit.xml

# List the catalogs
lancer-promptfoo redteam plugins
lancer-promptfoo redteam strategies

# Validate config + credentials
lancer-promptfoo validate --config promptfooconfig.yaml

# Self-test the whole matrix offline (no API keys)
lancer-promptfoo self-test

# MCP server (stdio) for Claude Code / Cursor / Continue
lancer-promptfoo mcp
```

Open the GUI by running `lancer-promptfoo` with no arguments.

## Build

### Windows
```
build.bat
```
Output: `build\lancer-promptfoo.exe`. Packaged dist: `build-windows.bat`.

### Linux / macOS
```bash
./build-linux.sh
```
Output: `build/lancer-promptfoo`.

Headless / CI without a display:
```bash
cargo build --release --no-default-features   # CLI-only (no GUI)
```

### Requirements
- [Rust / Cargo](https://rustup.rs)
- (Windows) Visual C++ Build Tools 2019/2022
- (Linux GUI) standard X11/Wayland + GL dev libs for `eframe`

## Environment variables

Honors the upstream `PROMPTFOO_*` surface: `PROMPTFOO_DISABLE_REDTEAM_REMOTE_GENERATION`,
`PROMPTFOO_CACHE_PATH`, `PROMPTFOO_LOG_DIR`, `PROMPTFOO_CONFIG_DIR`,
`PROMPTFOO_SHARING_ENDPOINT`, plus provider keys (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`,
`GOOGLE_API_KEY`, `AWS_*`, `AZURE_*`, `GROQ_API_KEY`, `HF_TOKEN`, …).

## Architecture

```
src/
├── core/          # GUI-agnostic engine
│   ├── types.rs traits.rs            # domain model + Provider/Plugin/Strategy/Assertion traits
│   ├── config.rs                     # promptfooconfig.yaml schema + generator
│   ├── providers/ plugins/ strategies/ assertions/   # registries (const spec tables)
│   ├── presets.rs presets_data.rs    # framework preset / collection expansion
│   ├── eval.rs redteam.rs engine.rs  # the two engines + shared plumbing
│   ├── report/                       # json csv yaml html xml sarif junit markdown terminal
│   ├── cache.rs env.rs               # cache + PROMPTFOO_* env handling
│   └── selftest.rs                   # parity self-test matrix
├── cli/           # full CLI surface (every upstream subcommand + flag)
├── gui/           # native egui desktop frontend + cross-tool server
├── integration/   # subprocess bridge to upstream promptfoo + HTTP capture shim
├── mcp.rs share.rs auth.rs store.rs  # MCP server, cloud share/auth, local eval store
└── main.rs lib.rs bin/cli.rs

tests/upstream_parity/   # parity matrix (catalog floors, flag parsing, formats, aliases)
```

## Status

Functional Rust reimplementation. The deterministic engine (encoders, assertions, report
formats, registries, offline self-test) runs without any network. Live LLM calls use the
built-in OpenAI-compatible / Anthropic / HTTP transports; providers without a built-in
transport are catalogued and routed through the generic `http`/gateway providers.
