mod cli;
mod discovery;
mod filter;
mod output;
mod session;

use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use rayon::prelude::*;
use std::path::{Path, PathBuf};

use cli::Cli;
use filter::SortKey;

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let now = Utc::now();

    let root = discovery::projects_root(cli.projects_dir.as_deref())?;
    if !root.is_dir() {
        eprintln!(
            "error: Claude projects directory not found at {}",
            root.display()
        );
        std::process::exit(2);
    }

    let recursive = !cli.no_recursive;
    let target = discovery::resolve_target(cli.path.as_deref())?;
    let encoded_target = discovery::encode_path(&target);

    // 1. candidate project dirs (loose encoded pre-filter)
    let mut dirs: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(&root)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if cli.global || discovery::dir_matches(&encoded_target, name, recursive) {
            dirs.push(path);
        }
    }

    // 2. gather .jsonl files
    let mut files: Vec<PathBuf> = Vec::new();
    for dir in &dirs {
        let rd = match std::fs::read_dir(dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                files.push(path);
            }
        }
    }

    // 3. parse in parallel, 4. verify real cwd is under target
    let mut sessions: Vec<session::Session> = files
        .par_iter()
        .filter_map(|p| match session::parse_session(p) {
            Ok(Some(s)) => Some(s),
            Ok(None) => None,
            Err(e) => {
                eprintln!("warning: skipping {}: {e}", p.display());
                None
            }
        })
        .filter(|s| {
            if cli.global {
                return true;
            }
            if s.cwd.is_empty() {
                return false;
            }
            discovery::path_under(&target, Path::new(&s.cwd), recursive)
        })
        .collect();

    // show a single session's transcript (--limit/-n tails the last N messages)
    if let Some(query) = &cli.show {
        return show_session(&sessions, query, cli.json, cli.limit);
    }

    // 5. time filtering (period first, explicit since/until override)
    let (mut since, mut until) = (None, None);
    if let Some(p) = &cli.period {
        let (s, u) = filter::period_bounds(p, now)?;
        since = s;
        until = u;
    }
    if let Some(s) = &cli.since {
        since = Some(filter::parse_when(s, now)?);
    }
    if let Some(u) = &cli.until {
        until = Some(filter::parse_when(u, now)?);
    }
    filter::time_filter(&mut sessions, since, until);

    // 6. fuzzy search (score order)
    let searched = cli.search.is_some();
    if let Some(term) = &cli.search {
        sessions = filter::search(sessions, term);
    }

    // 7. sort
    match cli.sort {
        Some(key) => filter::sort_sessions(&mut sessions, key, cli.reverse),
        None => {
            if !searched {
                filter::sort_sessions(&mut sessions, SortKey::Active, cli.reverse);
            } else if cli.reverse {
                sessions.reverse();
            }
        }
    }

    // 8. limit
    if let Some(n) = cli.limit {
        sessions.truncate(n);
    }

    // 9. output
    if cli.json {
        println!("{}", output::render_json(&sessions)?);
    } else {
        println!("{}", output::render_table(&sessions, now));
    }
    Ok(())
}

/// Find one session by id / short id / name and print its transcript (or raw jsonl for --json).
fn show_session(
    sessions: &[session::Session],
    query: &str,
    json: bool,
    limit: Option<usize>,
) -> Result<()> {
    let q = query.to_lowercase();
    let matches: Vec<&session::Session> = sessions
        .iter()
        .filter(|s| {
            s.session_id == query
                || s.session_id.starts_with(query)
                || s.short == query
                || s.name.to_lowercase().contains(&q)
        })
        .collect();
    match matches.as_slice() {
        [] => {
            eprintln!("error: no session matching '{query}' in scope (try -g for all projects)");
            std::process::exit(2);
        }
        [s] => {
            if json {
                println!("{}", output::render_transcript_json(s, limit)?);
            } else {
                println!("{}", output::render_transcript(s, limit)?);
            }
            Ok(())
        }
        many => {
            eprintln!(
                "error: '{query}' matches {} sessions; narrow it down:",
                many.len()
            );
            for s in many {
                eprintln!("  {}  {}", s.short, s.name.replace('\n', " "));
            }
            std::process::exit(2);
        }
    }
}
