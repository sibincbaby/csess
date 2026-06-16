---
name: csess
description: Use when you need to find Claude Code sessions for a folder — list session names, last-active times, message counts, and paths. Fast; prefer --json for parsing.
---

# Looking up Claude sessions with `csess`

`csess` lists Claude Code sessions for a folder and its subfolder projects.

## When to use
- "What sessions exist for this project?"
- "Find my session about X" → `csess -s X --json`
- "Which session was most recently active?" → `csess --json -n 1`

## How to call
Always prefer `--json` when you will parse the result.

```bash
csess --json                 # current dir + subfolders
csess /path/to/proj --json   # a specific tree
csess --global --json        # all projects
csess -s "auth" --json       # fuzzy search
csess --period today --json  # time-filtered
```

## Output (JSON array)
Each item: `session_id`, `short`, `name`, `cwd`, `last_active`, `created`,
`message_count`, `git_branch`, `version`, `size_bytes`, `file_path`.

Sorted newest-first by default. Use `--sort` and `-n/--limit` to refine.

## Install
1. Binary: `cargo install --git https://github.com/sibincbaby/csess`
2. This skill: copy this file to `~/.claude/skills/csess/SKILL.md` so `/csess` is available.
