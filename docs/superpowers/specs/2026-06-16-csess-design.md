# csess — Claude Session Lister (Design)

**Date:** 2026-06-16
**Status:** Approved (pending spec review)

## Purpose

A fast, reliable Rust CLI that lists Claude Code sessions for a folder and its
subfolder projects. Primary consumer is **Claude itself** — invoked with `--json`
to discover session details (name, last-active time, message count, path) in
seconds. A bundled `SKILL.md` teaches an AI agent when and how to call it.

## Background: how Claude stores sessions

- Each session is one `*.jsonl` file under `~/.claude/projects/<encoded-cwd>/`.
- The project directory name is the working directory with `/` replaced by `-`
  (e.g. `/home/sibin/my-works/tiptestapp` → `-home-sibin-my-works-tiptestapp`).
  This encoding is **lossy** — real folder names containing `-` are ambiguous.
- The filename (a UUID) is the `sessionId`.
- Each data line carries a `cwd` field (the real absolute path), `timestamp`,
  `gitBranch`, `version`, `type`, etc. Some sessions include a `summary`-type entry.

## Scope & non-goals

In scope: listing, filtering, fuzzy search, sorting, table + JSON output,
recursive subfolder discovery, cross-distro distribution.

Non-goals: editing/deleting sessions, reading full conversation content,
resuming sessions, any network calls.

## Architecture

Single static binary `csess`, installable on `PATH` so any Claude session can
call it. Distribution: `cargo install`, prebuilt static (musl) binaries via
GitHub Releases, and a one-line install script.

### Modules

- `main.rs` — clap arg parsing, orchestration, exit codes.
- `discovery.rs` — locate projects root; enumerate project dirs; fast-decode dir
  names (`-` → `/`) to pre-filter against the target path; scope logic
  (cwd-tree default / specific path / `--global`).
- `session.rs` — `Session` struct; parse one `.jsonl` file; extract fields;
  verify the real `cwd` is under the target (resolves the dashed-name ambiguity).
- `filter.rs` — time-window filtering, fuzzy search + ranking, sorting.
- `output.rs` — table renderer (default) and JSON serializer.

### Data flow

```
args → resolve target path + scope
     → discover candidate project dirs (decode name + prefix filter)
     → parallel (rayon) parse each *.jsonl → Session, verify cwd under target
     → filter (period/since/until/search)
     → sort (default: last-active, newest first)
     → render (table | --json)
```

## Path resolution (the reliable part)

1. **Fast pre-filter:** decode each project dir name and keep those whose decoded
   path equals the target or starts with `target + "/"` (recursive). `--global`
   keeps all.
2. **Verify on match:** read the in-file `cwd` and confirm it is the target or a
   descendant. This corrects false positives/negatives from dashed folder names.

## `Session` fields

| field | source | notes |
|-------|--------|-------|
| `session_id` | filename (UUID) | also a `short` (first 8 chars) for the table |
| `name` | `summary` entry if present, else first real `user` prompt | skip meta/command/attachment/sidechain lines; truncated in table, full in JSON |
| `cwd` | first `cwd` field in file | real project path |
| `last_active` | file mtime | fast & reliable; table = relative ("2h ago"), JSON = RFC3339 |
| `created` | first `timestamp` in file (fallback: file ctime) | |
| `message_count` | count of `user`/`assistant` turns | counted during the single read |
| `git_branch` | first `gitBranch` field | may be empty |
| `version` | first `version` field | Claude Code version |
| `size_bytes` | file size | |
| `file_path` | absolute path | for tooling |

## Search (best-fit convenience)

`--search TERM` performs **fuzzy** matching (via `fuzzy-matcher`, SkimMatcherV2,
smart-case) across the session `name` and `cwd`. Matches are ranked by fuzzy
score; with `--sort` unspecified and a search active, results are ordered by score
(then last-active). Substring queries naturally match as a subset of fuzzy, so a
single flag covers both casual and precise lookups.

## CLI

```
csess [PATH] [options]

  PATH                folder to scan (default: current directory)
  -g, --global        all Claude projects on the machine
  -R, --no-recursive  exact folder only (recursive is the default)
  -s, --search TERM   fuzzy match on session name / path
  --since WHEN        lower time bound: 2026-06-01 | 7d | 24h | 30m
  --until WHEN        upper time bound (same formats)
  --period P          today | yesterday | week | month
  --sort KEY          active | created | name | messages | size  (default: active)
  -r, --reverse       reverse the sort (default order: newest first)
  -n, --limit N       cap number of results
  --json              machine-readable JSON output
      --projects-dir DIR   override projects root (hidden; for tests)
  -h, --help   -V, --version
```

## Output

- **Table (default):** aligned columns via `comfy-table` —
  `SHORT | NAME | LAST ACTIVE | MSGS | BRANCH | PATH`. NAME/PATH truncated to fit.
  A trailing summary line: `N sessions`.
- **JSON (`--json`):** a JSON array of full `Session` objects (all fields, untruncated,
  timestamps RFC3339). Always valid JSON, even for zero results (`[]`).

## Error handling

- Missing `~/.claude/projects` → clear stderr message, exit code **2**.
- Permission-denied / unreadable file → skip with a stderr warning, continue.
- Malformed JSON line → skip the line, continue parsing the file.
- Unparseable session (no usable data) → skip with warning.
- Zero results → empty table / `[]`, exit **0**.
- Invalid arguments/usage → exit **1** (clap default).

Reliability principle: one bad file never aborts the whole run.

## Performance

- Each file is read once, streaming line-by-line.
- Files parsed in parallel with `rayon`.
- Expected dataset: a few hundred files → results in well under a second.

## Testing

- **Unit:** path decode/match incl. dashed-folder edge cases; `--since`/`--period`
  parsing (`7d`, `24h`, `today`); name resolution from sample JSONL lines;
  sort and filter logic; fuzzy ranking ordering.
- **Integration:** `tests/fixtures/` containing a fake projects root with tiny
  `.jsonl` files; injected via the hidden `--projects-dir` flag; `assert_cmd` +
  `predicates` assert both table and `--json` output.

## Dependencies

Runtime: `clap` (derive), `serde`, `serde_json`, `chrono`, `rayon`,
`comfy-table`, `fuzzy-matcher`, `dirs`, `anyhow`.
Dev: `assert_cmd`, `tempfile`, `predicates`.

## Distribution & project hygiene

- **Repo:** new public crate at `/home/sibin/my-works/csess`, published under the
  **personal** GitHub account (`sibincbaby`), HTTPS remote.
- **License:** MIT (`LICENSE`).
- **Versioning:** SemVer, starting `0.1.0`; `CHANGELOG.md` (Keep a Changelog);
  git tags `vX.Y.Z`.
- **CI (GitHub Actions):**
  - `ci.yml` — on push/PR: `cargo fmt --check`, `cargo clippy -D warnings`,
    `cargo test` (stable, Ubuntu).
  - `release.yml` — on tag `v*`: build `x86_64-unknown-linux-musl` (static,
    portable across distros) + `x86_64-unknown-linux-gnu`, attach tarballs to a
    GitHub Release, generate release notes from `CHANGELOG.md`.
- **README.md:** badges, feature list, install options (`cargo install csess` /
  prebuilt binary / install script), usage + examples, JSON schema, the AI/skill
  integration section, contributing, license.
- **SKILL.md:** frontmatter (`name`, `description`) + guidance for an AI agent on
  when to use `csess`, preferring `--json`, with concrete examples and install note.
- **Repo files:** `.gitignore` (`/target`), `rustfmt.toml`, `Cargo.toml` metadata
  (description, repository, license, keywords, categories) so it is publishable to
  crates.io later.

## Portability notes

- Pure Rust, no system/C dependencies → builds and runs on any modern Linux.
- musl static build has zero shared-lib requirements (works across Ubuntu/Debian/
  Alpine/etc.).
- Uses `dirs` to resolve the home directory rather than hardcoding `/home`.
