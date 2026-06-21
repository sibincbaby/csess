# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
