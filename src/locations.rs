use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::config::{
    expand_tilde, get_app_config_dir, get_config_file, read_key_value, set_key_value,
};

/// Returns the path to the `locations` file in the given config directory (or default).
pub fn get_locations_file(config_dir: Option<&Path>) -> Option<PathBuf> {
    let locations_file = "locations";
    match config_dir {
        Some(p) => Some(p.join(locations_file)),
        None => get_app_config_dir().map(|p| p.join(locations_file)),
    }
}

/// Retrieves the configured parent directory for a repository from `locations` or `config` files.
pub fn get_configured_parent(repo: &Path, config_dir: Option<&Path>) -> Option<PathBuf> {
    let repo_str = repo.to_string_lossy().to_string();
    let repo_canonical = repo.canonicalize().ok();

    let check_file = |file: &Path| -> Option<PathBuf> {
        if let Ok(entries) = read_key_value(file) {
            for (k, val) in entries {
                if k == repo_str
                    || repo_canonical.as_ref().is_some_and(|c| {
                        Path::new(&k).canonicalize().ok().as_ref() == Some(c)
                    })
                {
                    let expanded = expand_tilde(&val);
                    let trimmed = expanded.to_string_lossy().trim_end_matches('/').to_string();
                    return Some(PathBuf::from(trimmed));
                }
            }
        }
        None
    };

    if let Some(locations_file) = get_locations_file(config_dir) {
        if let Some(parent) = check_file(&locations_file) {
            return Some(parent);
        }
    }

    if let Some(config_file) = get_config_file(config_dir) {
        if let Some(parent) = check_file(&config_file) {
            return Some(parent);
        }
    }

    None
}

/// Saves the configured safe parent directory for a repository in the `locations` file.
pub fn save_configured_parent(
    repo: &Path,
    safe_parent: &Path,
    config_dir: Option<&Path>,
) -> io::Result<()> {
    let locations_file = match get_locations_file(config_dir) {
        Some(f) => f,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine config directory",
            ))
        }
    };
    let repo_str = repo.to_string_lossy().to_string();
    let parent_str = safe_parent.to_string_lossy().to_string();
    set_key_value(&locations_file, &repo_str, &parent_str)
}

/// Checks whether a given repository path is considered "unsuitable" for creating worktrees alongside it
/// (e.g. within hidden directories like `~/.local` or `.hidden`, or when parent is root `/` or non-writable).
pub fn is_unsuitable_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    if path_str.contains("/.") || path_str.starts_with('.') {
        return true;
    }

    let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => return true,
    };

    if parent == Path::new("/") {
        return true;
    }

    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        if let Ok(c_path) = CString::new(parent.as_os_str().as_bytes()) {
            unsafe extern "C" {
                fn access(path: *const std::ffi::c_char, amode: std::ffi::c_int) -> std::ffi::c_int;
            }
            const W_OK: std::ffi::c_int = 2;
            unsafe {
                if access(c_path.as_ptr(), W_OK) != 0 {
                    return true;
                }
            }
        }
    }
    #[cfg(not(unix))]
    {
        if let Ok(m) = parent.metadata() {
            if m.permissions().readonly() {
                return true;
            }
        }
    }

    false
}

/// Determines the worktree parent directory (`dir_gwt`) for a repository.
///
/// Prompts the user for a safe directory if the repository path is unsuitable.
pub fn determine_dir_gwt<R: io::BufRead>(
    target_repo: &Path,
    config_dir: Option<&Path>,
    mut prompt_reader: Option<&mut R>,
) -> Option<PathBuf> {
    let dir_name = target_repo
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repo");
    let gwt_dir_name = format!("gwt-{dir_name}");

    if let Some(safe_parent) = get_configured_parent(target_repo, config_dir) {
        return Some(safe_parent.join(&gwt_dir_name));
    }

    if is_unsuitable_path(target_repo) {
        eprintln!(
            "gwt: repository is in an unsuitable location ({})",
            target_repo.display()
        );

        let input_opt = match prompt_reader.as_mut() {
            Some(reader) => {
                eprint!("Enter safe parent directory for worktrees (e.g. ~/projects): ");
                let _ = io::stderr().flush();
                let mut user_input = String::new();
                if reader.read_line(&mut user_input).is_ok() {
                    Some(user_input)
                } else {
                    None
                }
            }
            None => inquire::Text::new("Enter safe parent directory for worktrees (e.g. ~/projects):")
                .prompt()
                .ok(),
        };

        if let Some(user_input) = input_opt {
            let trimmed = user_input.trim();
            if !trimmed.is_empty() {
                let safe_parent = expand_tilde(trimmed);
                let _ = save_configured_parent(target_repo, &safe_parent, config_dir);
                return Some(safe_parent.join(&gwt_dir_name));
            }
        }

        return None;
    }

    let parent = target_repo
        .parent()
        .unwrap_or_else(|| Path::new("."));
    Some(parent.join(&gwt_dir_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_locations_file_and_configured_parent() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_loc_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_path = Path::new("/Users/user/projects/myrepo");
        let safe_parent = Path::new("/Users/user/custom-parent");

        assert_eq!(get_configured_parent(repo_path, Some(&temp_dir)), None);

        save_configured_parent(repo_path, safe_parent, Some(&temp_dir)).unwrap();
        assert_eq!(
            get_configured_parent(repo_path, Some(&temp_dir)),
            Some(safe_parent.to_path_buf())
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_is_unsuitable_path() {
        assert!(is_unsuitable_path(Path::new("/home/user/.local/share/repo")));
        assert!(is_unsuitable_path(Path::new(".hidden_repo")));
        assert!(is_unsuitable_path(Path::new("./my_repo")));
        assert!(is_unsuitable_path(Path::new("/root_repo")));
    }
}
