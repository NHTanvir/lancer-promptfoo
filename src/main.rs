//! `lancer-promptfoo` — primary binary. Opens the native GUI by default; falls
//! through to the CLI surface when a subcommand (or `--cli`) is given.

fn main() -> anyhow::Result<()> {
    lancer_promptfoo::run_main()
}
