use std::fmt;
use std::io;
use std::path::Path;

use crate::ide::{get_configured_ide, set_configured_ide};

/// Error types that can occur during the `ide` command.
#[derive(Debug)]
pub enum IdeError {
    /// Could not determine config directory (exit code 1).
    ConfigDirNotFound,
    /// An I/O error occurred (exit code 1).
    Io(io::Error),
}

impl IdeError {
    /// Returns the associated process exit code matching `gwt` specifications.
    pub fn exit_code(&self) -> i32 {
        match self {
            IdeError::ConfigDirNotFound => 1,
            IdeError::Io(_) => 1,
        }
    }
}

impl fmt::Display for IdeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdeError::ConfigDirNotFound => write!(f, "could not determine config directory"),
            IdeError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for IdeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            IdeError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for IdeError {
    fn from(err: io::Error) -> Self {
        if err.kind() == io::ErrorKind::NotFound
            && err.to_string().contains("Could not determine config directory")
        {
            IdeError::ConfigDirNotFound
        } else {
            IdeError::Io(err)
        }
    }
}

/// Returns the current configured IDE.
pub fn get_ide(config_dir: Option<&Path>) -> String {
    get_configured_ide(config_dir)
}

/// Sets the configured IDE in the configuration file.
pub fn set_ide(ide: &str, config_dir: Option<&Path>) -> Result<String, IdeError> {
    set_configured_ide(ide, config_dir)?;
    Ok(format!("gwt: set ide to {ide}"))
}

/// Executes the `ide` command given CLI arguments.
///
/// If `args` is empty, returns the configured IDE (defaults to `nvim`).
/// If `args` is non-empty, configures `ide` to the given value and returns the confirmation message.
pub fn execute_ide(args: &[String], config_dir: Option<&Path>) -> Result<String, IdeError> {
    if args.is_empty() {
        Ok(get_ide(config_dir))
    } else {
        let ide_val = args.join(" ");
        set_ide(&ide_val, config_dir)
    }
}

/// Runs the `ide` command and prints output to stdout.
pub fn ide_and_print(args: &[String]) -> Result<(), IdeError> {
    let output = execute_ide(args, None)?;
    println!("{output}");
    Ok(())
}

/// Runs the `ide` command with the provided argument slice.
pub fn run(args: &[String]) -> Result<(), IdeError> {
    ide_and_print(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_get_ide_default() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_ide_cmd_def_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let orig = std::env::var("GWT_IDE").ok();
        unsafe { std::env::remove_var("GWT_IDE") };

        let ide = execute_ide(&[], Some(&temp_dir)).unwrap();
        assert_eq!(ide, "nvim");

        if let Some(v) = orig {
            unsafe { std::env::set_var("GWT_IDE", v) };
        }
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_get_ide_from_env() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_ide_cmd_env_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let orig = std::env::var("GWT_IDE").ok();
        unsafe { std::env::set_var("GWT_IDE", "vim") };

        let ide = execute_ide(&[], Some(&temp_dir)).unwrap();
        assert_eq!(ide, "vim");

        match orig {
            Some(v) => unsafe { std::env::set_var("GWT_IDE", v) },
            None => unsafe { std::env::remove_var("GWT_IDE") },
        }
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_set_and_get_ide() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_ide_cmd_set_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Set IDE to code
        let out_set = execute_ide(&["code".to_string()], Some(&temp_dir)).unwrap();
        assert_eq!(out_set, "gwt: set ide to code");

        // Verify getting IDE returns code
        let out_get = execute_ide(&[], Some(&temp_dir)).unwrap();
        assert_eq!(out_get, "code");

        // Update IDE to cursor
        let out_update = execute_ide(&["cursor".to_string()], Some(&temp_dir)).unwrap();
        assert_eq!(out_update, "gwt: set ide to cursor");

        let out_get2 = execute_ide(&[], Some(&temp_dir)).unwrap();
        assert_eq!(out_get2, "cursor");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_set_multi_word_ide() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_ide_cmd_multi_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let out_set = execute_ide(
            &["code".to_string(), "--wait".to_string()],
            Some(&temp_dir),
        )
        .unwrap();
        assert_eq!(out_set, "gwt: set ide to code --wait");

        let out_get = execute_ide(&[], Some(&temp_dir)).unwrap();
        assert_eq!(out_get, "code --wait");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_set_ide_none() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_ide_cmd_none_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let out_set = execute_ide(&["none".to_string()], Some(&temp_dir)).unwrap();
        assert_eq!(out_set, "gwt: set ide to none");

        let out_get = execute_ide(&[], Some(&temp_dir)).unwrap();
        assert_eq!(out_get, "none");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ide_error_exit_codes() {
        assert_eq!(IdeError::ConfigDirNotFound.exit_code(), 1);
        assert_eq!(
            IdeError::Io(io::Error::new(io::ErrorKind::Other, "io error")).exit_code(),
            1
        );
        assert_eq!(
            IdeError::ConfigDirNotFound.to_string(),
            "could not determine config directory"
        );
    }
}
