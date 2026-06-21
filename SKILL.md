---
name: csess
description: Use when you need to find Claude Code sessions for a folder, or read a full session transcript by id — list names/times/sizes, or fetch the whole conversation as JSON. Fast; prefer --json for parsing.
---

# Looking up Claude sessions with `csess`

`csess` does two things: (1) **list** Claude Code sessions for a folder and its
subprojects, and (2) **show** one session's full conversation transcript.

## When to use
- "What sessions exist for this project?" → `csess --json`
- "Find my session about X" → `csess -s X --json`
- "Which session was most recently active?" → `csess --json -n 1`
- "Show / open / read the full conversation for session <id>" → `csess -g --show <id> --json`
- "Give me the last 20 messages of that session" → `csess -g --show <id> -n 20 --json`

## Listing
Always prefer `--json` when you will parse the result.

```bash
csess --json                 # current dir + subfolders
csess /path/to/proj --json   # a specific tree
csess --global --json        # all projects (-g)
csess -s "auth" --json       # fuzzy search on name + path
csess --period today --json  # time-filtered (today|yesterday|week|month)
csess --since 7d --until 2026-06-01 --json   # explicit bounds (30m|24h|7d|YYYY-MM-DD)
csess --sort size --json     # sort: active(default)|created|name|messages|size; -r reverses
csess -n 10 --json           # limit to N results
```

### List output (JSON array)
Each item: `session_id`, `short`, `name`, `cwd`, `last_active`, `created`,
`message_count`, `git_branch`, `version`, `size_bytes`, `file_path`.
Sorted newest-first by default. (Human table also shows a SIZE column.)

## Showing a full transcript
`--show <ID_OR_NAME>` prints one session's whole conversation. Match is by full
id, short id (first 8 chars), or a substring of the name. Use `-g` so it finds
the session regardless of which project folder it lives in. If the query matches
more than one session it lists the candidates and exits non-zero — narrow it.

```bash
csess -g --show 09f8a9f7              # human-readable transcript
csess -g --show 09f8a9f7 --json      # structured, for an agent/UI
csess -g --show 09f8a9f7 -n 20 --json  # only the last 20 messages (tail)
```

- **Human** (no `--json`): a header, then each message as `## role · timestamp`
  with content flattened; tool activity shown as `[tool_use: NAME] {input}`,
  `[tool_result] …`, `[thinking] …`.
- **JSON** (`--json`): for rendering a chat UI. Shape:
  ```jsonc
  {
    "session_id", "name", "cwd", "created", "git_branch", "version", "message_count",
    "messages": [
      {
        "role": "user" | "assistant",
        "timestamp": "RFC3339",
        "uuid": "…",          // stable key
        "sidechain": true,     // present only for subagent turns
        "content": …           // raw Anthropic content: a string, OR an array of blocks
      }
    ]
  }
  ```
  `content` blocks are raw and untouched: `{type:"text",text}`,
  `{type:"thinking",thinking}`, `{type:"tool_use",id,name,input}`,
  `{type:"tool_result",tool_use_id,content,is_error}`. Pair a result to its call
  via `tool_use_id` ↔ `tool_use.id`. `-n N` keeps the last N messages.

## Install
1. Binary: `cargo install --git https://github.com/sibincbaby/csess`
2. This skill: copy this file to `~/.claude/skills/csess/SKILL.md` so `/csess` is available.
