# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] - 2026-06-27

### Added
- `--role user|assistant` filters a transcript to one side. Works with `--show`
  and with `--grep`.
- `--grep <text>` searches message text (case-insensitive substring over the
  flattened, human-readable content — tool calls included, matching what you
  see). With `--show` it filters that one transcript; without `--show` it scans
  every in-scope session in parallel and prints each match's session plus its
  matching messages as one-line snippets. Honours `-g`/`--period`/`--since` and
  `--json` (output shape `{ "schema_version": 1, "matches": [...] }`).

## [0.4.0] - 2026-06-25

### Added
- `schema_version` field on all `--json` output (currently `1`). Downstream
  tools can pin to it; field renames/removals/retypes bump it, additive fields
  don't.

### Changed
- **Breaking:** the list `--json` output is now an object
  `{ "schema_version": 1, "sessions": [...] }` instead of a bare array. Consumers
  that parsed the top-level array must read the `sessions` key. The `--show
  --json` transcript is unchanged apart from gaining `schema_version` (additive).

## [0.3.0] - 2026-06-21

### Added
- `--show --before <uuid>` pages a transcript backwards: it returns only the
  messages older than the given message uuid, then `-n` keeps the last N of
  those. A uuid cursor (not an offset) for scroll-up / lazy-load chat UIs —
  immune to index drift when the active session appends new turns.

### Changed
- `--show` and `message_count` now skip the synthetic resume entries Claude Code
  writes to its own log (the `isMeta` "Continue from where you left off." and the
  `model: "<synthetic>"` "No response requested." pairs). Transcripts and counts
  now match what the Claude Code UI displays instead of over-reporting.

## [0.2.1] - 2026-06-21

### Added
- `csess --version` now reports the Claude Code version this release was tested
  against (e.g. `0.2.1 (verified with Claude Code 2.1.185)`), so `.jsonl` format
  drift in a future Claude Code release can be traced to a known-good baseline.

## [0.2.0] - 2026-06-21

### Added
- `--show <id-or-name>` prints a single session's full transcript. Matches by
  full id, short id, or a name substring; ambiguous matches list candidates and
  exit non-zero. Use `-g` to find a session in any project.
- `--show --json` emits a structured transcript (session metadata +
  `messages[]` with `role`, `timestamp`, `uuid`, `sidechain`, and the raw
  Anthropic `content` blocks) for re-rendering a conversation in another UI.
  Plain `--show` prints a human-readable view.
- `-n/--limit` applied to `--show` keeps only the last N messages (tail).
- SIZE column in the listing table (size was already in `--json`).

## [0.1.1] - 2026-06-19

### Fixed
- Session name now reads the `ai-title` line Claude Code writes, matching the
  native `/resume` picker. Name precedence is `aiTitle` → `summary` →
  first real prompt.
- Skip `<ide_...>` selections and `Base directory for this skill:` injections
  when falling back to the first prompt for a name.

## [0.1.0] - 2026-06-16

### Added
- List Claude Code sessions for a folder and its subfolder projects.
- Recursive discovery with loose encoded pre-filter + strict `cwd` verification.
- Fuzzy search (`--search`), time filters (`--since`/`--until`/`--period`),
  sorting (`--sort`), `--limit`, `--global`.
- Table (default) and `--json` output.
