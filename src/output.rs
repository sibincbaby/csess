use crate::session::Session;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use comfy_table::presets::UTF8_BORDERS_ONLY;
use comfy_table::Table;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};

/// Pretty JSON array of full session objects (always valid, even when empty).
pub fn render_json(sessions: &[Session]) -> Result<String> {
    Ok(serde_json::to_string_pretty(sessions)?)
}

/// Human-readable aligned table with a trailing count.
pub fn render_table(sessions: &[Session], now: DateTime<Utc>) -> String {
    let mut table = Table::new();
    table.load_preset(UTF8_BORDERS_ONLY);
    table.set_header(vec![
        "SHORT",
        "NAME",
        "LAST ACTIVE",
        "MSGS",
        "SIZE",
        "BRANCH",
        "PATH",
    ]);
    for s in sessions {
        let name = truncate(&s.name.replace('\n', " "), 50);
        table.add_row(vec![
            s.short.clone(),
            name,
            relative_time(s.last_active, now),
            s.message_count.to_string(),
            human_size(s.size_bytes),
            s.git_branch.clone(),
            truncate(&s.cwd, 45),
        ]);
    }
    format!("{table}\n{} sessions", sessions.len())
}

/// One message in a session transcript, shaped for re-rendering a chat UI.
/// `content` is the raw Anthropic content (a string, or the array of
/// text/thinking/tool_use/tool_result blocks) exactly as Claude Code logged it.
#[derive(Serialize)]
pub struct Message {
    pub role: &'static str,
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub sidechain: bool,
    pub content: Value,
}

/// Full session object plus its messages, for `--show --json`.
#[derive(Serialize)]
struct Transcript<'a> {
    session_id: &'a str,
    name: &'a str,
    cwd: &'a str,
    created: Option<DateTime<Utc>>,
    git_branch: &'a str,
    version: &'a str,
    message_count: usize,
    messages: Vec<Message>,
}

/// Walk the .jsonl and pull out user/assistant messages with their timestamps.
/// `limit` keeps only the last N messages (the tail) when set.
fn collect_messages(path: &str, limit: Option<usize>) -> Result<Vec<Message>> {
    let file = fs::File::open(path).with_context(|| format!("opening {path}"))?;
    let mut msgs = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let role = match v.get("type").and_then(|t| t.as_str()) {
            Some("user") => "user",
            Some("assistant") => "assistant",
            _ => continue,
        };
        let content = match v.get("message").and_then(|m| m.get("content")) {
            Some(c) => c.clone(),
            None => continue,
        };
        let timestamp = v
            .get("timestamp")
            .and_then(|x| x.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc));
        let uuid = v
            .get("uuid")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());
        let sidechain = v
            .get("isSidechain")
            .and_then(|b| b.as_bool())
            .unwrap_or(false);
        msgs.push(Message {
            role,
            timestamp,
            uuid,
            sidechain,
            content,
        });
    }
    if let Some(n) = limit {
        let start = msgs.len().saturating_sub(n);
        msgs.drain(..start);
    }
    Ok(msgs)
}

/// Full conversation transcript for a single session, header followed by each message.
pub fn render_transcript(s: &Session, limit: Option<usize>) -> Result<String> {
    let mut out = format!("# {}\n", s.name.replace('\n', " "));
    out.push_str(&format!("id: {}\n", s.session_id));
    out.push_str(&format!("cwd: {}\n", s.cwd));
    if let Some(c) = s.created {
        out.push_str(&format!("created: {}\n", c.to_rfc3339()));
    }
    out.push_str(&format!("messages: {}\n", s.message_count));

    for m in collect_messages(&s.file_path, limit)? {
        let text = match flatten_content(&m.content) {
            Some(t) if !t.trim().is_empty() => t,
            _ => continue,
        };
        match m.timestamp {
            Some(ts) => out.push_str(&format!(
                "\n## {} · {}\n{}\n",
                m.role,
                ts.to_rfc3339(),
                text
            )),
            None => out.push_str(&format!("\n## {}\n{}\n", m.role, text)),
        }
    }
    Ok(out)
}

/// Structured JSON for a single session's transcript (session metadata + messages).
pub fn render_transcript_json(s: &Session, limit: Option<usize>) -> Result<String> {
    let t = Transcript {
        session_id: &s.session_id,
        name: &s.name,
        cwd: &s.cwd,
        created: s.created,
        git_branch: &s.git_branch,
        version: &s.version,
        message_count: s.message_count,
        messages: collect_messages(&s.file_path, limit)?,
    };
    Ok(serde_json::to_string_pretty(&t)?)
}

/// Flatten content into readable text; tool calls/results/thinking become bracketed markers.
/// Used only for the human-readable text view — JSON keeps the raw content blocks.
fn flatten_content(content: &Value) -> Option<String> {
    if let Some(s) = content.as_str() {
        return Some(s.to_string());
    }
    let mut parts = Vec::new();
    for block in content.as_array()? {
        match block.get("type").and_then(|t| t.as_str()) {
            Some("text") => {
                if let Some(t) = block.get("text").and_then(|x| x.as_str()) {
                    parts.push(t.to_string());
                }
            }
            Some("thinking") => {
                let t = block.get("thinking").and_then(|x| x.as_str()).unwrap_or("");
                parts.push(format!("[thinking]\n{t}"));
            }
            Some("tool_use") => {
                let name = block.get("name").and_then(|x| x.as_str()).unwrap_or("tool");
                let input = block
                    .get("input")
                    .map(|i| serde_json::to_string(i).unwrap_or_default())
                    .unwrap_or_default();
                parts.push(format!("[tool_use: {name}] {input}"));
            }
            Some("tool_result") => {
                let body = tool_result_text(block.get("content"));
                parts.push(format!("[tool_result] {body}"));
            }
            _ => {}
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

/// Flatten a tool_result's `content` (string, or array of text blocks) into one string.
fn tool_result_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// Bytes as a compact human-readable size (e.g. 12.3K, 4.5M).
fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "K", "M", "G"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes}{}", UNITS[0])
    } else {
        format!("{size:.1}{}", UNITS[unit])
    }
}

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_string();
    }
    let cut = max.saturating_sub(1);
    let head: String = chars[..cut].iter().collect();
    format!("{head}…")
}

fn relative_time(then: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let d = now.signed_duration_since(then);
    let secs = d.num_seconds();
    if secs < 0 {
        return "in future".to_string();
    }
    if secs < 60 {
        return "just now".to_string();
    }
    if d.num_minutes() < 60 {
        return format!("{}m ago", d.num_minutes());
    }
    if d.num_hours() < 24 {
        return format!("{}h ago", d.num_hours());
    }
    let days = d.num_days();
    if days < 30 {
        return format!("{days}d ago");
    }
    if days < 365 {
        return format!("{}mo ago", days / 30);
    }
    format!("{}y ago", days / 365)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample() -> Session {
        Session {
            session_id: "11111111-x".into(),
            short: "11111111".into(),
            name: "Build the thing".into(),
            cwd: "/home/sibin/demo".into(),
            last_active: Utc.with_ymd_and_hms(2026, 6, 16, 10, 0, 0).unwrap(),
            created: None,
            message_count: 4,
            git_branch: "main".into(),
            version: "1.0".into(),
            size_bytes: 10,
            file_path: "/x".into(),
        }
    }

    #[test]
    fn json_contains_name() {
        let out = render_json(&[sample()]).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v[0]["name"], "Build the thing");
    }

    #[test]
    fn table_has_short_and_footer() {
        let now = Utc.with_ymd_and_hms(2026, 6, 16, 12, 0, 0).unwrap();
        let out = render_table(&[sample()], now);
        assert!(out.contains("11111111"));
        assert!(out.contains("Build the thing"));
        assert!(out.contains("1 sessions"));
    }

    #[test]
    fn human_size_scales() {
        assert_eq!(human_size(512), "512B");
        assert_eq!(human_size(1536), "1.5K");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0M");
    }

    #[test]
    fn truncate_and_relative() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello", 3), "he…");
        let now = Utc.with_ymd_and_hms(2026, 6, 16, 12, 0, 0).unwrap();
        let then = Utc.with_ymd_and_hms(2026, 6, 16, 10, 0, 0).unwrap();
        assert_eq!(relative_time(then, now), "2h ago");
    }
}
