use std::io;
use std::path::Path;

use crate::config::{get_config_file, get_key_value, set_key_value};

/// Default fallback IDE when not configured.
pub const DEFAULT_IDE: &str = "nvim";

/// Returns the configured IDE name or command.
///
/// Priority:
/// 1. `ide` setting in the `config` file
/// 2. `GWT_IDE` environment variable
/// 3. Default fallback: `"nvim"`
pub fn get_configured_ide(config_dir: Option<&Path>) -> String {
    if let Some(config_file) = get_config_file(config_dir) {
        if let Ok(Some(val)) = get_key_value(&config_file, "ide") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    if let Ok(env_ide) = std::env::var("GWT_IDE") {
        let trimmed = env_ide.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    DEFAULT_IDE.to_string()
}

/// Sets the configured IDE in the configuration file.
pub fn set_configured_ide(ide: &str, config_dir: Option<&Path>) -> io::Result<()> {
    let config_file = match get_config_file(config_dir) {
        Some(file) => file,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine config directory",
            ))
        }
    };
    set_key_value(&config_file, "ide", ide)
}

/// Resolves the effective IDE command to use given an optional CLI override (`--ide`).
/// Returns `None` if the effective IDE is `"none"`, or `Some(command)` otherwise.
pub fn resolve_ide_command(override_ide: Option<&str>, config_dir: Option<&Path>) -> Option<String> {
    let ide = override_ide
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| get_configured_ide(config_dir));

    if ide.eq_ignore_ascii_case("none") {
        None
    } else {
        Some(ide)
    }
}

/// Launches the configured IDE in the specified directory, unless the effective IDE is "none".
pub fn launch_ide(
    override_ide: Option<&str>,
    dir: &Path,
    config_dir: Option<&Path>,
) -> io::Result<()> {
    if let Some(ide_cmd) = resolve_ide_command(override_ide, config_dir) {
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&ide_cmd)
            .current_dir(dir)
            .status()?;
        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("IDE command '{ide_cmd}' failed with status: {status}"),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_get_configured_ide_default() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_ide_mod_def_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let orig = std::env::var("GWT_IDE").ok();
        unsafe { std::env::remove_var("GWT_IDE") };

        let ide = get_configured_ide(Some(&temp_dir));
        assert_eq!(ide, DEFAULT_IDE);

        if let Some(v) = orig {
            unsafe { std::env::set_var("GWT_IDE", v) };
        }
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_get_configured_ide_from_env() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_ide_mod_env_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let orig = std::env::var("GWT_IDE").ok();
        unsafe { std::env::set_var("GWT_IDE", "vim") };

        let ide = get_configured_ide(Some(&temp_dir));
        assert_eq!(ide, "vim");

        match orig {
            Some(v) => unsafe { std::env::set_var("GWT_IDE", v) },
            None => unsafe { std::env::remove_var("GWT_IDE") },
        }
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_set_and_get_configured_ide() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_ide_mod_set_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        set_configured_ide("code", Some(&temp_dir)).unwrap();
        let ide = get_configured_ide(Some(&temp_dir));
        assert_eq!(ide, "code");

        set_configured_ide("cursor", Some(&temp_dir)).unwrap();
        let updated_ide = get_configured_ide(Some(&temp_dir));
        assert_eq!(updated_ide, "cursor");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_ide_command() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_ide_mod_resolve_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Default resolution
        let orig = std::env::var("GWT_IDE").ok();
        unsafe { std::env::remove_var("GWT_IDE") };

        assert_eq!(resolve_ide_command(None, Some(&temp_dir)), Some("nvim".to_string()));
        assert_eq!(resolve_ide_command(Some("code"), Some(&temp_dir)), Some("code".to_string()));
        assert_eq!(resolve_ide_command(Some("none"), Some(&temp_dir)), None);
        assert_eq!(resolve_ide_command(Some("NONE"), Some(&temp_dir)), None);

        // When configured to none in config file
        set_configured_ide("none", Some(&temp_dir)).unwrap();
        assert_eq!(resolve_ide_command(None, Some(&temp_dir)), None);
        // Override takes precedence over config
        assert_eq!(resolve_ide_command(Some("zed"), Some(&temp_dir)), Some("zed".to_string()));

        if let Some(v) = orig {
            unsafe { std::env::set_var("GWT_IDE", v) };
        }
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_launch_ide() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_ide_mod_launch_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // When IDE is "none", no command is executed
        let res = launch_ide(Some("none"), &temp_dir, Some(&temp_dir));
        assert!(res.is_ok());

        // When a custom IDE command is executed
        let test_file = temp_dir.join("launched.txt");
        let res = launch_ide(Some("touch launched.txt"), &temp_dir, Some(&temp_dir));
        assert!(res.is_ok());
        assert!(test_file.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
