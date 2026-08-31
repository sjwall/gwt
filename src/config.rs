use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Returns the configuration directory,
/// adhering to `${XDG_CONFIG_HOME:-$HOME/.config}/gwt`.
pub fn get_app_config_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.trim().is_empty() {
            return Some(PathBuf::from(xdg).join("gwt"));
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.trim().is_empty() {
            return Some(PathBuf::from(home).join(".config").join("gwt"));
        }
    }
    None
}

/// Expands leading `~` or `~/` in a path string to `$HOME`.
pub fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home);
        }
    } else if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

/// Reads trimmed non-empty, non-comment lines from a file.
pub fn read_lines(file_path: &Path) -> io::Result<Vec<String>> {
    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(file_path)?;
    let mut lines = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        lines.push(trimmed.to_string());
    }
    Ok(lines)
}

/// Appends a line to a file if it is not already present. Creates parent directories and file if needed.
/// Returns `true` if the line was newly added, `false` if it was already present.
pub fn append_unique_line(file_path: &Path, line: &str) -> io::Result<bool> {
    if file_path.exists() {
        let content = fs::read_to_string(file_path)?;
        for existing in content.lines() {
            if existing.trim() == line.trim() {
                return Ok(false);
            }
        }
    } else if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;

    writeln!(file, "{}", line.trim())?;
    Ok(true)
}

/// Reads key-value pairs from a file (`KEY=VALUE` format), skipping comments and empty lines.
pub fn read_key_value(file_path: &Path) -> io::Result<Vec<(String, String)>> {
    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(file_path)?;
    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            entries.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    Ok(entries)
}

/// Retrieves the value of a key from a key-value configuration file.
pub fn get_key_value(file_path: &Path, key: &str) -> io::Result<Option<String>> {
    let entries = read_key_value(file_path)?;
    for (k, v) in entries {
        if k == key {
            return Ok(Some(v));
        }
    }
    Ok(None)
}

/// Sets a key-value pair in a file. Creates or updates the entry and preserves unrelated lines.
pub fn set_key_value(file_path: &Path, key: &str, value: &str) -> io::Result<()> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut lines = Vec::new();
    let mut found = false;

    if file_path.exists() {
        let content = fs::read_to_string(file_path)?;
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with('#') && trimmed.contains('=') {
                if let Some((k, _)) = trimmed.split_once('=') {
                    if k.trim() == key {
                        lines.push(format!("{key}={value}"));
                        found = true;
                        continue;
                    }
                }
            }
            lines.push(line.to_string());
        }
    }

    if !found {
        lines.push(format!("{key}={value}"));
    }

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(file_path)?;

    for line in lines {
        writeln!(file, "{line}")?;
    }

    Ok(())
}

/// Unsets a key from a key-value file. Returns `true` if removed, `false` if not found.
pub fn unset_key_value(file_path: &Path, key: &str) -> io::Result<bool> {
    if !file_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(file_path)?;
    let mut lines = Vec::new();
    let mut removed = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') && trimmed.contains('=') {
            if let Some((k, _)) = trimmed.split_once('=') {
                if k.trim() == key {
                    removed = true;
                    continue;
                }
            }
        }
        lines.push(line.to_string());
    }

    if removed {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(file_path)?;

        for line in lines {
            writeln!(file, "{line}")?;
        }
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde() {
        if let Ok(home) = std::env::var("HOME") {
            assert_eq!(expand_tilde("~"), PathBuf::from(&home));
            assert_eq!(
                expand_tilde("~/foo/bar"),
                PathBuf::from(&home).join("foo/bar")
            );
        }
        assert_eq!(expand_tilde("/var/repo"), PathBuf::from("/var/repo"));
    }

    #[test]
    fn test_read_lines_and_append_unique() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_generic_cfg_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let test_file = temp_dir.join("test_lines.txt");

        let added = append_unique_line(&test_file, "/path/one").unwrap();
        assert!(added);

        let added_dup = append_unique_line(&test_file, "/path/one").unwrap();
        assert!(!added_dup);

        let added_two = append_unique_line(&test_file, "/path/two").unwrap();
        assert!(added_two);

        let lines = read_lines(&test_file).unwrap();
        assert_eq!(
            lines,
            vec!["/path/one".to_string(), "/path/two".to_string()]
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_key_value_operations() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_generic_kv_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let kv_file = temp_dir.join("test_kv.conf");

        assert_eq!(get_key_value(&kv_file, "key1").unwrap(), None);

        set_key_value(&kv_file, "key1", "val1").unwrap();
        assert_eq!(
            get_key_value(&kv_file, "key1").unwrap(),
            Some("val1".to_string())
        );

        set_key_value(&kv_file, "key1", "val2").unwrap();
        assert_eq!(
            get_key_value(&kv_file, "key1").unwrap(),
            Some("val2".to_string())
        );

        set_key_value(&kv_file, "key2", "val3").unwrap();
        let entries = read_key_value(&kv_file).unwrap();
        assert_eq!(entries.len(), 2);

        let unset = unset_key_value(&kv_file, "key1").unwrap();
        assert!(unset);
        assert_eq!(get_key_value(&kv_file, "key1").unwrap(), None);
        assert_eq!(
            get_key_value(&kv_file, "key2").unwrap(),
            Some("val3".to_string())
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
