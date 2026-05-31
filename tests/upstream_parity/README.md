# Upstream parity matrix

This directory pins the upstream promptfoo surface that `lancer-promptfoo` must
keep matching. The test crate `tests/upstream_parity.rs` enforces it:

- **Catalog floors** — provider / plugin / strategy / assertion / preset /
  collection counts may never regress below their parity floors.
- **CLI flags** — every documented upstream subcommand + flag must parse.
- **Legacy aliases** — old strategy ids (`jailbreak:meta`, `camelCase`,
  `morse`, `emoji`, `composite`, …) must still resolve.
- **Preset / collection expansion** — every expanded id must be a real plugin.
- **Encoders** — known base64/hex/rot13/morse vectors.
- **Output formats** — json, yaml, csv, html, xml, sarif, junit, markdown, text
  all render.
- **Self-test** — the full offline matrix (`lancer-promptfoo self-test`) passes.

`flags.txt` is the canonical reference list of upstream flags tracked here.

Run with:

```
cargo test --test upstream_parity
```
