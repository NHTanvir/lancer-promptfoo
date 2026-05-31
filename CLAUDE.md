# CLAUDE.md

Guidance for Claude Code (and humans) working in this repository.

## What this is

`lancer-promptfoo` — A Rust reimplementation and native GUI of [promptfoo](https://promptfoo.dev), the LLM red-team and evaluation framework, built for the Lancer 2026 AI-era bug-bounty toolkit. It ships a native Rust engine covering eval, red-team with 157+ plugins, 30+ delivery strategies, 45+ assertion types, ~60 provider integrations, all upstream output formats, an MCP server mode, and an egui desktop GUI. It also supports an optional subprocess bridge to the upstream `promptfoo` CLI for exact upstream engine compatibility.

## Architecture

- `src/lib.rs` — Library entry point; all shared logic is exposed from here so both binaries can consume it.
- `src/main.rs` — Primary binary entry point; opens the native egui GUI by default, falls through to CLI when a subcommand or `--cli` is given.
- `src/bin/cli.rs` — Companion headless CLI-only binary (`lancer-promptfoo-cli`); never links the GUI crate.
- `src/cli/` — CLI surface: argument parsing, subcommand dispatch (mod.rs).
- `src/core/` — Core engine modules: `engine.rs` (eval/redteam orchestration), `eval.rs`, `redteam.rs`, `config.rs`, `types.rs`, `traits.rs`, `providers/` (~60 provider integrations), `plugins/` (157+ red-team plugins), `strategies/` (30+ delivery strategies), `assertions/` (45+ assertion types), `report/` (JSON/YAML/CSV/HTML/SARIF/JUnit/Markdown output), `presets.rs`/`presets_data.rs` (OWASP/MITRE/NIST/GDPR presets), `cache.rs`, `env.rs`, `selftest.rs`, `promptfoo_json.rs`.
- `src/gui/` — egui desktop frontend (gated behind the `gui` feature): `mod.rs`, `crosstool.rs` (cross-tool HTTP server for `/v1/state`, `/v1/import`, `/v1/export`).
- `src/integration/` — Subprocess bridge and shim: `mod.rs`, `promptfoo.rs`, `shim.rs`.
- `src/mcp.rs` — MCP server mode; exposes `eval`, `redteam_run`, and `assert` as tools (stdio transport).
- `src/auth.rs` — Cloud auth (`auth` / `share` subcommands, `PROMPTFOO_SHARING_ENDPOINT`).
- `src/share.rs` — Share/upload logic for cloud result sharing.
- `src/store.rs` — Persistent local state store.
- `tests/` — Integration test suite.

## Conventions

- Error propagation uses `anyhow::Result`; no `unwrap`/`expect` on fallible I/O.
- GUI is gated behind the default `gui` feature; `--no-default-features` builds a headless CLI binary.
- Library code lives in `src/lib.rs`; primary binary entry point in `src/main.rs`; headless CLI binary in `src/bin/cli.rs`.
- Provider integrations live under `src/core/providers/`; adding a new provider means implementing the shared trait defined in `src/core/traits.rs`.
- Plugin and strategy catalogs live under `src/core/plugins/` and `src/core/strategies/`; legacy IDs (e.g. `jailbreak:meta`, `camelCase`) are resolved via alias tables — do not remove alias entries.
- Output format writers live under `src/core/report/`; SARIF and JUnit outputs are used for CI gating and GitHub Code Scanning integration.

## Build / test / run

```
build.bat                   # Windows → build\lancer-promptfoo.exe
./build-linux.sh            # Linux/macOS
cargo build --release       # standard release build
cargo test                  # unit + integration tests
```

Headless / CI build (no GUI, no display required):
```
cargo build --release --no-default-features
```

## Repo notes

- `plan.md` is local-only and must NOT be committed. It is gitignored; if ever tracked, run `git rm --cached plan.md`.
- `.claude/` and `target/`/`build/`/`dist/` are local-only and gitignored.
