# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`csess` is a Rust CLI that lists Claude Code sessions for a folder and its subprojects by reading the session `.jsonl` logs under `~/.claude/projects`.

## Commands

```bash
cargo build                      # debug build
cargo build --release            # release (strip + lto, see Cargo.toml)
cargo test                       # unit tests (in-module) + integration tests (tests/cli.rs)
cargo test parse_session         # run tests matching a name
cargo test --test cli            # only the integration tests
cargo fmt                        # rustfmt (config: rustfmt.toml)
cargo clippy
cargo run -- <args>              # e.g. cargo run -- ~/my-works --period today --json
```

## Definition of done

Before considering any change complete, run all of these and ensure they pass:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

Treat clippy warnings as errors — fix them, don't silence them with `#[allow(...)]` unless there's a documented reason. Don't leave `dbg!`, stray `println!`/`eprintln!` (outside intended output paths), or commented-out code behind. "It compiles" is not done; the three checks above are.

## Architecture

The whole pipeline is the numbered steps in `run()` in `src/main.rs`; each module owns one stage.

**The core trick (`discovery.rs`):** Claude stores each project's sessions in a directory named by encoding the real cwd with `/` → `-` (e.g. `/home/sibin/my-works` → `-home-sibin-my-works`). This encoding is lossy/ambiguous — a real dir named `my-works-backup` and a subdir `my-works/backup` can collide. So discovery is **two-phase**:
1. `dir_matches` — loose, fast pre-filter on the encoded dir name. Deliberately *over-includes* dashed siblings.
2. `path_under` — strict, component-wise check against each session's real `cwd` (read from inside the `.jsonl`) to drop the false positives.

Never collapse these into one step or "fix" the over-inclusion in phase 1 — the encoded name genuinely can't distinguish the cases; only the real cwd inside the file can. The `path_under_excludes_dashed_sibling` test guards this.

**Parsing (`session.rs`):** `parse_session` scans a `.jsonl` line by line, tolerating malformed lines (skips them). It pulls `cwd`/`gitBranch`/`version`/first `timestamp` from whichever line has them first, counts user+assistant messages, and derives the display name as `summary` → else first real user prompt → else `(no name)`. `last_active` is the file mtime (not parsed from content). Files with no usable content return `Ok(None)`. `is_meta_text` skips `<command-*>`/`<bash-*>`/`Caveat:` lines so session names come from genuine prompts.

**Filtering/sorting (`filter.rs`):** Time bounds resolve as period preset first, then explicit `--since`/`--until` override (`parse_when` accepts `30m`/`24h`/`7d` or `YYYY-MM-DD`). Search is fuzzy (skim matcher) over `name + cwd`, returning score-ranked results; when `--search` is active and no explicit `--sort` is given, score order is preserved instead of re-sorting.

**Output (`output.rs`):** `--json` emits the full `Session` structs (serde); default is a `comfy-table` view with relative times and a trailing count.

Parsing runs in parallel across files via `rayon` (`par_iter` in `main.rs`); keep `parse_session` free of shared mutable state.

## Conventions

- **Errors:** use `anyhow::Result`; `main` prints `error: {e:#}` and exits non-zero. Per-file parse failures warn to stderr and are skipped, never fatal. Add context at boundaries with `.context(...)`/`.with_context(...)` rather than bubbling bare errors.
- **No `unwrap`/`expect` outside `#[cfg(test)]`.** In non-test code, propagate with `?` or contextualize via `anyhow`. `expect` is acceptable only for genuine invariants that cannot fail, and must carry a message explaining why.
- **Dependencies:** prefer what's already here (`rayon`, `serde`, `comfy-table`, the skim fuzzy matcher, `assert_cmd`, `anyhow`) and the standard library. Don't add a new crate without a clear reason; if you do, note why.
- **Toolchain:** edition and MSRV are whatever `Cargo.toml` declares — match them and don't use features newer than the pinned edition/MSRV. <!-- set/confirm the exact edition + MSRV here -->
- `--projects-dir` is a hidden flag that overrides `~/.claude/projects` — the integration tests in `tests/cli.rs` rely on it to seed a temp root. Don't change or remove flags that tests depend on without updating the tests.
- Each `src/*.rs` module keeps its own `#[cfg(test)]` unit tests; end-to-end behavior is covered in `tests/cli.rs` via `assert_cmd`.

<!--
Keep this file high-signal: it loads into context on every turn. When you refactor a
module, update its paragraph above so the guidance never drifts from the code — stale
instructions are worse than none.
-->