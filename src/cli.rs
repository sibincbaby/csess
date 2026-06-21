use crate::filter::SortKey;
use clap::Parser;

// `--version` reports the Claude Code version this release was tested against.
// Bump the literal below each release to the `version` field seen in recent
// session .jsonl files — lets us pin format drift if a future Claude Code
// release breaks parsing.
#[derive(Parser, Debug)]
#[command(
    name = "csess",
    version = concat!(
        env!("CARGO_PKG_VERSION"),
        " (verified with Claude Code 2.1.185)"
    ),
    about = "List Claude Code sessions for a folder and its subprojects"
)]
pub struct Cli {
    /// Folder to scan (default: current directory)
    pub path: Option<String>,

    /// List sessions across all Claude projects
    #[arg(short = 'g', long)]
    pub global: bool,

    /// Only the exact folder (default also scans subfolders)
    #[arg(short = 'R', long = "no-recursive")]
    pub no_recursive: bool,

    /// Fuzzy match on session name / path
    #[arg(short = 's', long)]
    pub search: Option<String>,

    /// Print the full transcript of one session (by id, short id, or name)
    #[arg(long, value_name = "ID_OR_NAME")]
    pub show: Option<String>,

    /// With --show: only messages older than this message uuid (scroll-up cursor)
    #[arg(long, value_name = "UUID", requires = "show")]
    pub before: Option<String>,

    /// Lower time bound: 2026-06-01 | 7d | 24h | 30m
    #[arg(long)]
    pub since: Option<String>,

    /// Upper time bound (same formats)
    #[arg(long)]
    pub until: Option<String>,

    /// today | yesterday | week | month
    #[arg(long)]
    pub period: Option<String>,

    /// Sort key (default: active)
    #[arg(long, value_enum)]
    pub sort: Option<SortKey>,

    /// Reverse sort order
    #[arg(short = 'r', long)]
    pub reverse: bool,

    /// Limit number of results
    #[arg(short = 'n', long)]
    pub limit: Option<usize>,

    /// Machine-readable JSON output
    #[arg(long)]
    pub json: bool,

    /// Override projects root (for testing)
    #[arg(long, hide = true)]
    pub projects_dir: Option<String>,
}
