use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub session_id: String,
    pub short: String,
    pub name: String,
    pub cwd: String,
    pub last_active: DateTime<Utc>,
    pub created: Option<DateTime<Utc>>,
    pub message_count: usize,
    pub git_branch: String,
    pub version: String,
    pub size_bytes: u64,
    pub file_path: String,
}

/// Parse a single session file. Returns Ok(None) when the file holds no session data.
pub fn parse_session(path: &Path) -> Result<Option<Session>> {
    let meta = fs::metadata(path)?;
    let size_bytes = meta.len();
    let last_active: DateTime<Utc> = meta.modified()?.into();

    let session_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut cwd = String::new();
    let mut git_branch = String::new();
    let mut version = String::new();
    let mut ai_title: Option<String> = None;
    let mut summary: Option<String> = None;
    let mut first_prompt: Option<String> = None;
    let mut created: Option<DateTime<Utc>> = None;
    let mut message_count = 0usize;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let typ = v.get("type").and_then(|t| t.as_str()).unwrap_or("");

        if is_synthetic_entry(&v) {
            continue;
        }

        if cwd.is_empty() {
            if let Some(c) = v.get("cwd").and_then(|c| c.as_str()) {
                cwd = c.to_string();
            }
        }
        if git_branch.is_empty() {
            if let Some(b) = v.get("gitBranch").and_then(|b| b.as_str()) {
                git_branch = b.to_string();
            }
        }
        if version.is_empty() {
            if let Some(ver) = v.get("version").and_then(|x| x.as_str()) {
                version = ver.to_string();
            }
        }
        if created.is_none() {
            if let Some(ts) = v.get("timestamp").and_then(|t| t.as_str()) {
                if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
                    created = Some(dt.with_timezone(&Utc));
                }
            }
        }

        match typ {
            "ai-title" => {
                if let Some(s) = v.get("aiTitle").and_then(|s| s.as_str()) {
                    if !s.trim().is_empty() {
                        ai_title = Some(s.trim().to_string());
                    }
                }
            }
            "summary" => {
                if let Some(s) = v.get("summary").and_then(|s| s.as_str()) {
                    if !s.trim().is_empty() {
                        summary = Some(s.trim().to_string());
                    }
                }
            }
            "user" => {
                message_count += 1;
                let is_sidechain = v
                    .get("isSidechain")
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);
                if first_prompt.is_none() && !is_sidechain {
                    if let Some(text) = extract_user_text(&v) {
                        let t = text.trim();
                        if !t.is_empty() && !is_meta_text(t) {
                            first_prompt = Some(t.to_string());
                        }
                    }
                }
            }
            "assistant" => {
                message_count += 1;
            }
            _ => {}
        }
    }

    if cwd.is_empty()
        && ai_title.is_none()
        && summary.is_none()
        && first_prompt.is_none()
        && message_count == 0
    {
        return Ok(None);
    }

    let name = ai_title
        .or(summary)
        .or(first_prompt)
        .unwrap_or_else(|| "(no name)".to_string());
    let short = session_id.chars().take(8).collect();

    Ok(Some(Session {
        session_id,
        short,
        name,
        cwd,
        last_active,
        created,
        message_count,
        git_branch,
        version,
        size_bytes,
        file_path: path.to_string_lossy().to_string(),
    }))
}

/// Pull text out of a user message whose content may be a string or text blocks.
fn extract_user_text(v: &serde_json::Value) -> Option<String> {
    let content = v.get("message")?.get("content")?;
    if let Some(s) = content.as_str() {
        return Some(s.to_string());
    }
    if let Some(arr) = content.as_array() {
        let mut out = String::new();
        for block in arr {
            if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(t) = block.get("text").and_then(|t| t.as_str()) {
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    out.push_str(t);
                }
            }
        }
        if !out.is_empty() {
            return Some(out);
        }
    }
    None
}

/// Synthetic resume placeholders Claude Code writes to the `.jsonl` itself on
/// non-interactive resume (`isMeta:true` "Continue from where you left off." and
/// the `model:"<synthetic>"` "No response requested." reply). The Claude UI hides
/// these; csess matches that and drops them from transcripts and counts.
pub(crate) fn is_synthetic_entry(v: &serde_json::Value) -> bool {
    v.get("isMeta").and_then(|b| b.as_bool()).unwrap_or(false)
        || v.get("message")
            .and_then(|m| m.get("model"))
            .and_then(|m| m.as_str())
            == Some("<synthetic>")
}

/// Heuristic: skip command/attachment/system meta messages when naming a session.
fn is_meta_text(t: &str) -> bool {
    t.starts_with("<command-")
        || t.starts_with("<local-command")
        || t.starts_with("<bash-")
        || t.starts_with("<ide_")
        || t.starts_with("Caveat:")
        || t.starts_with("Base directory for this skill:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_session_extracts_name_and_cwd() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir
            .path()
            .join("11111111-2222-3333-4444-555555555555.jsonl");
        fs::write(
            &p,
            concat!(
                "{\"type\":\"user\",\"cwd\":\"/home/sibin/proj\",\"gitBranch\":\"main\",\"version\":\"1.0\",\"timestamp\":\"2026-06-16T01:00:00.000Z\",\"message\":{\"role\":\"user\",\"content\":\"Hello world\"}}\n",
                "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"Hi\"}]}}\n",
            ),
        )
        .unwrap();
        let s = parse_session(&p).unwrap().unwrap();
        assert_eq!(s.name, "Hello world");
        assert_eq!(s.cwd, "/home/sibin/proj");
        assert_eq!(s.git_branch, "main");
        assert_eq!(s.message_count, 2);
        assert_eq!(s.short, "11111111");
    }

    #[test]
    fn parse_session_skips_synthetic_resume_entries() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir
            .path()
            .join("dddddddd-0000-0000-0000-000000000000.jsonl");
        fs::write(
            &p,
            concat!(
                "{\"type\":\"user\",\"cwd\":\"/x\",\"message\":{\"role\":\"user\",\"content\":\"real prompt\"}}\n",
                "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"reply\"}]}}\n",
                "{\"type\":\"user\",\"isMeta\":true,\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"text\",\"text\":\"Continue from where you left off.\"}]}}\n",
                "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"model\":\"<synthetic>\",\"content\":[{\"type\":\"text\",\"text\":\"No response requested.\"}]}}\n",
            ),
        )
        .unwrap();
        let s = parse_session(&p).unwrap().unwrap();
        assert_eq!(s.name, "real prompt");
        // only the two genuine messages are counted, synthetic pair dropped
        assert_eq!(s.message_count, 2);
    }

    #[test]
    fn parse_session_prefers_summary() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir
            .path()
            .join("aaaaaaaa-0000-0000-0000-000000000000.jsonl");
        fs::write(
            &p,
            concat!(
                "{\"type\":\"summary\",\"summary\":\"Refactor auth\"}\n",
                "{\"type\":\"user\",\"cwd\":\"/x\",\"message\":{\"role\":\"user\",\"content\":\"hi\"}}\n",
            ),
        )
        .unwrap();
        let s = parse_session(&p).unwrap().unwrap();
        assert_eq!(s.name, "Refactor auth");
    }

    #[test]
    fn parse_session_prefers_ai_title_over_summary_and_skips_skill_meta() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir
            .path()
            .join("cccccccc-0000-0000-0000-000000000000.jsonl");
        fs::write(
            &p,
            concat!(
                "{\"type\":\"user\",\"cwd\":\"/x\",\"message\":{\"role\":\"user\",\"content\":\"Base directory for this skill: /home/x\"}}\n",
                "{\"type\":\"summary\",\"summary\":\"some summary\"}\n",
                "{\"type\":\"ai-title\",\"aiTitle\":\"Review LWR 127047\"}\n",
            ),
        )
        .unwrap();
        let s = parse_session(&p).unwrap().unwrap();
        assert_eq!(s.name, "Review LWR 127047");
    }

    #[test]
    fn parse_session_skips_garbage_and_empty() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir
            .path()
            .join("bbbbbbbb-0000-0000-0000-000000000000.jsonl");
        fs::write(&p, "not json\n\n{ broken\n").unwrap();
        assert!(parse_session(&p).unwrap().is_none());
    }
}
