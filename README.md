# csess

Fast, reliable CLI that lists [Claude Code](https://claude.com/claude-code)
sessions for a folder and its subfolder projects. Built for speed — designed to
be called by Claude itself with `--json`.

## Why

Claude stores each session as a `*.jsonl` file under
`~/.claude/projects/<encoded-cwd>/`. `csess` resolves those into a readable list
(name, last-active, message count, path), correctly handling nested projects and
folder names containing dashes.

## Install

### From source (Cargo)
```bash
cargo install --git https://github.com/sibincbaby/csess
```

### Prebuilt binary
Download the latest static binary from
[Releases](https://github.com/sibincbaby/csess/releases), then:
```bash
tar xzf csess-*-linux-musl.tar.gz
sudo mv csess /usr/local/bin/
```

### Install script
```bash
curl -fsSL https://raw.githubusercontent.com/sibincbaby/csess/main/install.sh | bash
```

## Usage

```bash
csess                      # sessions for the current dir + subfolders
csess /home/me/projects    # a specific folder tree
csess --global             # every Claude project on the machine
csess -s auth              # fuzzy search name/path
csess --period today       # only today's sessions
csess --since 7d --sort messages
csess --json               # machine-readable output (for scripts / AI)
```

### Options
| Flag | Description |
|------|-------------|
| `PATH` | Folder to scan (default: cwd) |
| `-g, --global` | All Claude projects |
| `-R, --no-recursive` | Exact folder only |
| `-s, --search TERM` | Fuzzy match on name/path |
| `--since` / `--until` | `2026-06-01` \| `7d` \| `24h` \| `30m` |
| `--period` | `today` \| `yesterday` \| `week` \| `month` |
| `--sort` | `active` \| `created` \| `name` \| `messages` \| `size` |
| `-r, --reverse` | Reverse order |
| `-n, --limit N` | Cap results |
| `--json` | JSON output |

### JSON fields
`session_id`, `short`, `name`, `cwd`, `last_active` (RFC3339), `created`,
`message_count`, `git_branch`, `version`, `size_bytes`, `file_path`.

## For AI agents (Claude Code)

Two steps — install the binary, then install the skill (a root `SKILL.md` does
**not** auto-register; it must live under `~/.claude/skills/`):

```bash
# 1. binary
cargo install --git https://github.com/sibincbaby/csess

# 2. skill → enables /csess in Claude Code
mkdir -p ~/.claude/skills/csess
curl -fsSL https://raw.githubusercontent.com/sibincbaby/csess/main/SKILL.md \
  -o ~/.claude/skills/csess/SKILL.md
```

See [`SKILL.md`](./SKILL.md) for when and how an agent should call `csess`
(prefer `--json`).

## License
MIT © Sibin C Baby
