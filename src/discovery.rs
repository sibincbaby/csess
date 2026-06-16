use anyhow::Result;
use std::path::{Path, PathBuf};

/// Resolve the Claude projects root (or an explicit override for tests).
pub fn projects_root(override_dir: Option<&str>) -> Result<PathBuf> {
    if let Some(d) = override_dir {
        return Ok(PathBuf::from(d));
    }
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    Ok(home.join(".claude").join("projects"))
}

/// Resolve the target folder to an absolute, lightly-normalized path.
pub fn resolve_target(path_arg: Option<&str>) -> Result<PathBuf> {
    let raw = match path_arg {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir()?,
    };
    let abs = if raw.is_absolute() {
        raw
    } else {
        std::env::current_dir()?.join(raw)
    };
    Ok(normalize(&abs))
}

/// Strip `.` components and any trailing slash without touching symlinks.
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            std::path::Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Encode a real path the way Claude names project dirs: `/` -> `-`.
pub fn encode_path(path: &Path) -> String {
    path.to_string_lossy().replace('/', "-")
}

/// Loose pre-filter on encoded directory names. Recursive includes children.
/// Intentionally over-includes dashed siblings; `path_under` corrects later.
pub fn dir_matches(encoded_target: &str, dir_name: &str, recursive: bool) -> bool {
    if dir_name == encoded_target {
        return true;
    }
    if recursive {
        if let Some(rest) = dir_name.strip_prefix(encoded_target) {
            return rest.starts_with('-');
        }
    }
    false
}

/// Strict, component-wise check that `cwd` is the target or (if recursive) a descendant.
pub fn path_under(target: &Path, cwd: &Path, recursive: bool) -> bool {
    if cwd == target {
        return true;
    }
    if recursive {
        return cwd.starts_with(target);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_path_replaces_slashes() {
        assert_eq!(
            encode_path(Path::new("/home/sibin/my-works")),
            "-home-sibin-my-works"
        );
    }

    #[test]
    fn dir_matches_exact_and_child() {
        let t = "-home-sibin-my-works";
        assert!(dir_matches(t, "-home-sibin-my-works", true));
        assert!(dir_matches(t, "-home-sibin-my-works-tiptestapp", true));
        assert!(!dir_matches(t, "-home-sibin-other", true));
        assert!(!dir_matches(t, "-home-sibin-my-works-tiptestapp", false));
    }

    #[test]
    fn path_under_excludes_dashed_sibling() {
        let target = Path::new("/home/sibin/my-works");
        assert!(path_under(target, Path::new("/home/sibin/my-works"), true));
        assert!(path_under(
            target,
            Path::new("/home/sibin/my-works/tiptestapp"),
            true
        ));
        // The loose encoded pre-filter would include this; strict check must reject it:
        assert!(!path_under(
            target,
            Path::new("/home/sibin/my-works-backup"),
            true
        ));
        assert!(!path_under(
            target,
            Path::new("/home/sibin/my-works/tiptestapp"),
            false
        ));
    }
}
