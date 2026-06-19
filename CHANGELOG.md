# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
