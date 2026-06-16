use crate::session::Session;
use anyhow::Result;
use chrono::{DateTime, Utc};
use comfy_table::presets::UTF8_BORDERS_ONLY;
use comfy_table::Table;

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
            s.git_branch.clone(),
            truncate(&s.cwd, 45),
        ]);
    }
    format!("{table}\n{} sessions", sessions.len())
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
    fn truncate_and_relative() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello", 3), "he…");
        let now = Utc.with_ymd_and_hms(2026, 6, 16, 12, 0, 0).unwrap();
        let then = Utc.with_ymd_and_hms(2026, 6, 16, 10, 0, 0).unwrap();
        assert_eq!(relative_time(then, now), "2h ago");
    }
}
