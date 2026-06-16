use crate::session::Session;
use anyhow::Result;
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum SortKey {
    Active,
    Created,
    Name,
    Messages,
    Size,
}

/// Parse a relative (`30m`/`24h`/`7d`) or absolute (`YYYY-MM-DD`) time bound.
pub fn parse_when(s: &str, now: DateTime<Utc>) -> Result<DateTime<Utc>> {
    let s = s.trim();
    for (suffix, mult) in [("m", 1i64), ("h", 60), ("d", 1440)] {
        if let Some(num) = s.strip_suffix(suffix) {
            if let Ok(n) = num.parse::<i64>() {
                return Ok(now - Duration::minutes(n * mult));
            }
        }
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).unwrap()));
    }
    anyhow::bail!("invalid time value '{s}' (use e.g. 30m, 24h, 7d, or 2026-06-01)")
}

/// A (since, until) time-bound pair.
pub type Bounds = (Option<DateTime<Utc>>, Option<DateTime<Utc>>);

/// Named period presets -> (since, until) bounds.
pub fn period_bounds(p: &str, now: DateTime<Utc>) -> Result<Bounds> {
    let midnight =
        |dt: DateTime<Utc>| Utc.from_utc_datetime(&dt.date_naive().and_hms_opt(0, 0, 0).unwrap());
    match p {
        "today" => Ok((Some(midnight(now)), None)),
        "yesterday" => {
            let today = midnight(now);
            Ok((Some(today - Duration::days(1)), Some(today)))
        }
        "week" => Ok((Some(now - Duration::days(7)), None)),
        "month" => Ok((Some(now - Duration::days(30)), None)),
        other => anyhow::bail!("invalid period '{other}' (use today|yesterday|week|month)"),
    }
}

/// Retain sessions whose last_active falls within [since, until).
pub fn time_filter(
    sessions: &mut Vec<Session>,
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
) {
    sessions.retain(|s| {
        if let Some(start) = since {
            if s.last_active < start {
                return false;
            }
        }
        if let Some(end) = until {
            if s.last_active >= end {
                return false;
            }
        }
        true
    });
}

/// Fuzzy match on name + cwd, returning matches ordered by descending score.
pub fn search(sessions: Vec<Session>, term: &str) -> Vec<Session> {
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, Session)> = sessions
        .into_iter()
        .filter_map(|s| {
            let hay = format!("{} {}", s.name, s.cwd);
            matcher.fuzzy_match(&hay, term).map(|score| (score, s))
        })
        .collect();
    scored.sort_by_key(|x| std::cmp::Reverse(x.0));
    scored.into_iter().map(|(_, s)| s).collect()
}

/// Sort in place by key. Defaults: time/size/messages descending, name ascending.
pub fn sort_sessions(sessions: &mut [Session], key: SortKey, reverse: bool) {
    match key {
        SortKey::Active => sessions.sort_by_key(|s| std::cmp::Reverse(s.last_active)),
        SortKey::Created => sessions.sort_by_key(|s| std::cmp::Reverse(s.created)),
        SortKey::Name => sessions.sort_by_key(|s| s.name.to_lowercase()),
        SortKey::Messages => sessions.sort_by_key(|s| std::cmp::Reverse(s.message_count)),
        SortKey::Size => sessions.sort_by_key(|s| std::cmp::Reverse(s.size_bytes)),
    }
    if reverse {
        sessions.reverse();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 16, 12, 0, 0).unwrap()
    }

    #[test]
    fn parse_when_relative_and_absolute() {
        let n = now();
        assert_eq!(parse_when("24h", n).unwrap(), n - Duration::hours(24));
        assert_eq!(parse_when("7d", n).unwrap(), n - Duration::days(7));
        assert_eq!(parse_when("30m", n).unwrap(), n - Duration::minutes(30));
        assert_eq!(
            parse_when("2026-06-01", n).unwrap(),
            Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap()
        );
        assert!(parse_when("nonsense", n).is_err());
    }

    #[test]
    fn period_bounds_today_and_week() {
        let n = now();
        let (since, until) = period_bounds("today", n).unwrap();
        assert_eq!(
            since.unwrap(),
            Utc.with_ymd_and_hms(2026, 6, 16, 0, 0, 0).unwrap()
        );
        assert!(until.is_none());
        let (wsince, _) = period_bounds("week", n).unwrap();
        assert_eq!(wsince.unwrap(), n - Duration::days(7));
        assert!(period_bounds("decade", n).is_err());
    }

    fn sample(name: &str, cwd: &str) -> Session {
        Session {
            session_id: "id".into(),
            short: "id".into(),
            name: name.into(),
            cwd: cwd.into(),
            last_active: now(),
            created: None,
            message_count: 0,
            git_branch: String::new(),
            version: String::new(),
            size_bytes: 0,
            file_path: String::new(),
        }
    }

    #[test]
    fn search_filters_and_ranks() {
        let sessions = vec![
            sample("Refactor authentication", "/a"),
            sample("Update README", "/b"),
            sample("Add auth tests", "/c"),
        ];
        let out = search(sessions, "auth");
        assert!(out.len() == 2);
        assert!(out.iter().all(|s| s.name.to_lowercase().contains("auth")));
    }

    #[test]
    fn sort_by_name_and_reverse() {
        let mut sessions = vec![sample("Beta", "/b"), sample("Alpha", "/a")];
        sort_sessions(&mut sessions, SortKey::Name, false);
        assert_eq!(sessions[0].name, "Alpha");
        sort_sessions(&mut sessions, SortKey::Name, true);
        assert_eq!(sessions[0].name, "Beta");
    }
}
