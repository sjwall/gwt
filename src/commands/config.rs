use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use crate::config::{get_config_file, get_key_value, set_key_value, unset_key_value};

/// Error types that can occur during the `config` command.
#[derive(Debug)]
pub enum ConfigError {
    /// Invalid argument count for `gwt config get` (exit code 29).
    GetUsage,
    /// Specified configuration key not found in `gwt config get` (exit code 30).
    KeyNotFoundGet(String),
    /// Invalid argument count for `gwt config set` (exit code 31).
    SetUsage,
    /// Invalid argument count for `gwt config unset` (exit code 32).
    UnsetUsage,
    /// Specified configuration key not found in `gwt config <key>` (exit code 33).
    KeyNotFound(String),
    /// Could not determine config directory (exit code 1).
    ConfigDirNotFound,
    /// An I/O error occurred (exit code 1).
    Io(io::Error),
}

impl ConfigError {
    /// Returns the associated process exit code matching `gwt` specifications.
    pub fn exit_code(&self) -> i32 {
        match self {
            ConfigError::GetUsage => 29,
            ConfigError::KeyNotFoundGet(_) => 30,
            ConfigError::SetUsage => 31,
            ConfigError::UnsetUsage => 32,
            ConfigError::KeyNotFound(_) => 33,
            ConfigError::ConfigDirNotFound => 1,
            ConfigError::Io(_) => 1,
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::GetUsage => write!(f, "usage: gwt config get <key>"),
            ConfigError::KeyNotFoundGet(key) => write!(f, "config key '{key}' not found"),
            ConfigError::SetUsage => write!(f, "usage: gwt config set <key> <value>"),
            ConfigError::UnsetUsage => write!(f, "usage: gwt config unset <key>"),
            ConfigError::KeyNotFound(key) => write!(f, "config key '{key}' not found"),
            ConfigError::ConfigDirNotFound => write!(f, "could not determine config directory"),
            ConfigError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self {
        ConfigError::Io(err)
    }
}

/// Lists all configuration lines from the config file.
pub fn list_config_lines(config_dir: Option<&Path>) -> Result<Vec<String>, ConfigError> {
    let config_file = get_config_file(config_dir);
    let mut lines = Vec::new();

    if let Some(ref file_path) = config_file {
        if file_path.exists() {
            let content = fs::read_to_string(file_path)?;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                lines.push(trimmed.to_string());
            }
        }
    }

    Ok(lines)
}

/// Gets the value of a specific configuration key.
pub fn get_config(
    key: &str,
    config_dir: Option<&Path>,
    not_found_err: impl FnOnce(String) -> ConfigError,
) -> Result<String, ConfigError> {
    let config_file = get_config_file(config_dir).ok_or(ConfigError::ConfigDirNotFound)?;
    let val = get_key_value(&config_file, key)?;
    match val {
        Some(v) if !v.trim().is_empty() => Ok(v),
        _ => Err(not_found_err(key.to_string())),
    }
}

/// Sets a configuration key to the given value in the config file.
pub fn set_config(
    key: &str,
    value: &str,
    config_dir: Option<&Path>,
) -> Result<String, ConfigError> {
    let config_file = get_config_file(config_dir).ok_or(ConfigError::ConfigDirNotFound)?;
    set_key_value(&config_file, key, value)?;
    Ok(format!("gwt: set {key} to {value}"))
}

/// Unsets a configuration key from the config file.
pub fn unset_config(key: &str, config_dir: Option<&Path>) -> Result<String, ConfigError> {
    let config_file = get_config_file(config_dir).ok_or(ConfigError::ConfigDirNotFound)?;
    let _ = unset_key_value(&config_file, key)?;
    Ok(format!("gwt: unset {key}"))
}

/// Executes the `config` command given CLI arguments, returning lines to be printed to stdout.
pub fn execute_config(
    args: &[String],
    config_dir: Option<&Path>,
) -> Result<Vec<String>, ConfigError> {
    if args.is_empty() {
        return list_config_lines(config_dir);
    }

    match args[0].as_str() {
        "get" => {
            if args.len() != 2 {
                return Err(ConfigError::GetUsage);
            }
            let val = get_config(&args[1], config_dir, ConfigError::KeyNotFoundGet)?;
            Ok(vec![val])
        }
        "set" => {
            if args.len() < 3 {
                return Err(ConfigError::SetUsage);
            }
            let key = &args[1];
            let val = args[2..].join(" ");
            let msg = set_config(key, &val, config_dir)?;
            Ok(vec![msg])
        }
        "unset" | "--unset" | "remove" | "rm" => {
            if args.len() != 2 {
                return Err(ConfigError::UnsetUsage);
            }
            let key = &args[1];
            let msg = unset_config(key, config_dir)?;
            Ok(vec![msg])
        }
        _ => {
            if args.len() == 1 {
                let key = &args[0];
                let val = get_config(key, config_dir, ConfigError::KeyNotFound)?;
                Ok(vec![val])
            } else {
                let key = &args[0];
                let val = args[1..].join(" ");
                let msg = set_config(key, &val, config_dir)?;
                Ok(vec![msg])
            }
        }
    }
}

/// Runs the `config` command and prints output to stdout.
pub fn run_config(args: &[String]) -> Result<(), ConfigError> {
    let lines = execute_config(args, None)?;
    for line in lines {
        println!("{line}");
    }
    Ok(())
}

/// Default entrypoint for the `config` command.
pub fn run(args: &[String]) -> Result<(), ConfigError> {
    run_config(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_list_config_empty() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_cfg_list_empty_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let lines = list_config_lines(Some(&temp_dir)).unwrap();
        assert!(lines.is_empty());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_list_config_with_entries() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_cfg_list_entries_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let config_file = temp_dir.join("config");
        fs::write(&config_file, "# Comment\nkey1=val1\nkey2=val2\n").unwrap();

        let lines = list_config_lines(Some(&temp_dir)).unwrap();
        assert_eq!(
            lines,
            vec!["key1=val1".to_string(), "key2=val2".to_string()]
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_execute_config_get() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_cfg_exec_get_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let config_file = temp_dir.join("config");
        fs::write(&config_file, "my_key=my_value\n").unwrap();

        // gwt config get my_key
        let out = execute_config(
            &["get".to_string(), "my_key".to_string()],
            Some(&temp_dir),
        )
        .unwrap();
        assert_eq!(out, vec!["my_value".to_string()]);

        // gwt config get nonexistent -> exit code 30
        let err = execute_config(
            &["get".to_string(), "nonexistent".to_string()],
            Some(&temp_dir),
        )
        .unwrap_err();
        assert_eq!(err.exit_code(), 30);
        assert_eq!(err.to_string(), "config key 'nonexistent' not found");

        // gwt config get (missing arg) -> exit code 29
        let err_usage = execute_config(&["get".to_string()], Some(&temp_dir)).unwrap_err();
        assert_eq!(err_usage.exit_code(), 29);
        assert_eq!(err_usage.to_string(), "usage: gwt config get <key>");

        // gwt config get a b -> exit code 29
        let err_extra = execute_config(
            &["get".to_string(), "a".to_string(), "b".to_string()],
            Some(&temp_dir),
        )
        .unwrap_err();
        assert_eq!(err_extra.exit_code(), 29);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_execute_config_set() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_cfg_exec_set_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // gwt config set k v
        let out = execute_config(
            &["set".to_string(), "k".to_string(), "v".to_string()],
            Some(&temp_dir),
        )
        .unwrap();
        assert_eq!(out, vec!["gwt: set k to v".to_string()]);

        // Verify set worked
        let val = execute_config(&["get".to_string(), "k".to_string()], Some(&temp_dir)).unwrap();
        assert_eq!(val, vec!["v".to_string()]);

        // gwt config set with multiple words
        let out_multi = execute_config(
            &[
                "set".to_string(),
                "msg".to_string(),
                "hello".to_string(),
                "world".to_string(),
            ],
            Some(&temp_dir),
        )
        .unwrap();
        assert_eq!(out_multi, vec!["gwt: set msg to hello world".to_string()]);

        let val_multi =
            execute_config(&["get".to_string(), "msg".to_string()], Some(&temp_dir)).unwrap();
        assert_eq!(val_multi, vec!["hello world".to_string()]);

        // gwt config set (missing args) -> exit code 31
        let err_usage0 = execute_config(&["set".to_string()], Some(&temp_dir)).unwrap_err();
        assert_eq!(err_usage0.exit_code(), 31);
        assert_eq!(err_usage0.to_string(), "usage: gwt config set <key> <value>");

        let err_usage1 = execute_config(
            &["set".to_string(), "only_key".to_string()],
            Some(&temp_dir),
        )
        .unwrap_err();
        assert_eq!(err_usage1.exit_code(), 31);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_execute_config_unset() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_cfg_exec_unset_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let config_file = temp_dir.join("config");
        fs::write(&config_file, "k1=v1\nk2=v2\n").unwrap();

        // gwt config unset k1
        let out = execute_config(
            &["unset".to_string(), "k1".to_string()],
            Some(&temp_dir),
        )
        .unwrap();
        assert_eq!(out, vec!["gwt: unset k1".to_string()]);

        // Verify k1 is unset
        let err = execute_config(&["get".to_string(), "k1".to_string()], Some(&temp_dir)).unwrap_err();
        assert_eq!(err.exit_code(), 30);

        // Verify k2 is preserved
        let out_k2 = execute_config(&["get".to_string(), "k2".to_string()], Some(&temp_dir)).unwrap();
        assert_eq!(out_k2, vec!["v2".to_string()]);

        // gwt config --unset k2
        let out_alias = execute_config(
            &["--unset".to_string(), "k2".to_string()],
            Some(&temp_dir),
        )
        .unwrap();
        assert_eq!(out_alias, vec!["gwt: unset k2".to_string()]);

        // gwt config remove
        let out_remove = execute_config(
            &["remove".to_string(), "k3".to_string()],
            Some(&temp_dir),
        )
        .unwrap();
        assert_eq!(out_remove, vec!["gwt: unset k3".to_string()]);

        // gwt config rm
        let out_rm = execute_config(
            &["rm".to_string(), "k4".to_string()],
            Some(&temp_dir),
        )
        .unwrap();
        assert_eq!(out_rm, vec!["gwt: unset k4".to_string()]);

        // Usage errors -> exit code 32
        let err_usage0 = execute_config(&["unset".to_string()], Some(&temp_dir)).unwrap_err();
        assert_eq!(err_usage0.exit_code(), 32);
        assert_eq!(err_usage0.to_string(), "usage: gwt config unset <key>");

        let err_usage_extra = execute_config(
            &["unset".to_string(), "a".to_string(), "b".to_string()],
            Some(&temp_dir),
        )
        .unwrap_err();
        assert_eq!(err_usage_extra.exit_code(), 32);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_execute_config_shorthand() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_cfg_exec_short_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // gwt config foo bar (set shorthand)
        let out_set = execute_config(
            &["foo".to_string(), "bar".to_string()],
            Some(&temp_dir),
        )
        .unwrap();
        assert_eq!(out_set, vec!["gwt: set foo to bar".to_string()]);

        // gwt config foo (get shorthand)
        let out_get = execute_config(&["foo".to_string()], Some(&temp_dir)).unwrap();
        assert_eq!(out_get, vec!["bar".to_string()]);

        // gwt config nonexistent (get shorthand missing) -> exit code 33
        let err = execute_config(&["nonexistent".to_string()], Some(&temp_dir)).unwrap_err();
        assert_eq!(err.exit_code(), 33);
        assert_eq!(err.to_string(), "config key 'nonexistent' not found");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_config_error_exit_codes() {
        assert_eq!(ConfigError::GetUsage.exit_code(), 29);
        assert_eq!(ConfigError::KeyNotFoundGet("k".into()).exit_code(), 30);
        assert_eq!(ConfigError::SetUsage.exit_code(), 31);
        assert_eq!(ConfigError::UnsetUsage.exit_code(), 32);
        assert_eq!(ConfigError::KeyNotFound("k".into()).exit_code(), 33);
        assert_eq!(ConfigError::ConfigDirNotFound.exit_code(), 1);
        assert_eq!(
            ConfigError::Io(io::Error::new(io::ErrorKind::Other, "io error")).exit_code(),
            1
        );
    }
}
