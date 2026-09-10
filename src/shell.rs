use std::path::Path;

/// Zsh completion script definition for `gwt` matching `_gwt`.
pub const ZSH_COMPLETION: &str = include_str!("../_gwt");

/// Shell wrapper script for `gwt`.
///
/// This wrapper can be sourced directly or evaluated in user shell profiles:
/// `source /path/to/_gwt_wrapper` or `eval "$(gwt --shell-wrapper)"`.
pub const SHELL_WRAPPER: &str = r#"# Shell wrapper for gwt (Git WorkTree Helper)
# Enables changing directory in the host shell for directory-changing commands.

unalias gwt 2>/dev/null || true

gwt() {
  local gwt_bin="gwt"
  if ! command -v "$gwt_bin" >/dev/null 2>&1; then
    if command -v gwt-bin >/dev/null 2>&1; then
      gwt_bin="gwt-bin"
    fi
  fi

  local cd_file
  cd_file=$(mktemp 2>/dev/null || mktemp -t gwt_cd 2>/dev/null)

  if [ -n "$cd_file" ]; then
    GWT_CD_FILE="$cd_file" command "$gwt_bin" "$@"
    local ret=$?
    if [ -s "$cd_file" ]; then
      local target
      target=$(cat "$cd_file")
      rm -f "$cd_file"
      if [ $ret -eq 0 ] && [ -n "$target" ]; then
        if [ -d "$target" ]; then
          builtin cd "$target" || return 14
        else
          return 14
        fi
      fi
    else
      rm -f "$cd_file"
    fi
    return $ret
  else
    case "$1" in
      cd|switch|s|main|m|M|Main|agent|a|add|pull|p|remove|rm|migrate)
        local target
        target=$(command "$gwt_bin" "$@") || return $?
        if [ -n "$target" ] && [ -d "$target" ]; then
          builtin cd "$target" || return 14
        fi
        ;;
      *)
        if [ $# -gt 0 ] && [ "${1#-}" = "$1" ] && [ "$1" != "list" ] && [ "$1" != "ls" ] && [ "$1" != "track" ] && [ "$1" != "t" ] && [ "$1" != "config" ] && [ "$1" != "ide" ] && [ "$1" != "skills" ] && [ "$1" != "completion" ] && [ "$1" != "completions" ] && [ "$1" != "autocomplete" ] && [ "$1" != "help" ]; then
          local target
          target=$(command "$gwt_bin" "$@") || return $?
          if [ -n "$target" ] && [ -d "$target" ]; then
            builtin cd "$target" || return 14
          fi
        else
          command "$gwt_bin" "$@"
        fi
        ;;
    esac
  fi
}

# Autocompletion for zsh
if [ -n "$ZSH_VERSION" ]; then
  _gwt_comp_bin="gwt"
  if ! command -v "$_gwt_comp_bin" >/dev/null 2>&1; then
    if command -v gwt-bin >/dev/null 2>&1; then
      _gwt_comp_bin="gwt-bin"
    fi
  fi
  if command -v "$_gwt_comp_bin" >/dev/null 2>&1; then
    eval "$("$_gwt_comp_bin" completion zsh 2>/dev/null || "$_gwt_comp_bin" --completion zsh 2>/dev/null)"
  fi
  unset _gwt_comp_bin
fi
"#;

/// Notifies a parent shell wrapper of a target directory to `cd` into.
///
/// If the `GWT_CD_FILE` environment variable is set, the path is written to the indicated file.
pub fn notify_cd_target(path: &Path) {
    if let Ok(cd_file) = std::env::var("GWT_CD_FILE") {
        let path_str = path.to_string_lossy();
        let _ = std::fs::write(cd_file, path_str.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_zsh_completion_constant() {
        assert!(ZSH_COMPLETION.contains("#compdef gwt"));
        assert!(ZSH_COMPLETION.contains("_gwt()"));
        assert!(ZSH_COMPLETION.contains("_gwt_comp_worktrees"));
        assert!(ZSH_COMPLETION.contains("_gwt_comp_repositories"));
    }

    #[test]
    fn test_shell_wrapper_contains_essential_elements() {
        assert!(SHELL_WRAPPER.contains("gwt()"));
        assert!(SHELL_WRAPPER.contains("GWT_CD_FILE"));
        assert!(SHELL_WRAPPER.contains("builtin cd"));
        assert!(SHELL_WRAPPER.contains("return 14"));
        assert!(SHELL_WRAPPER.contains("ZSH_VERSION"));
        assert!(SHELL_WRAPPER.contains("completion zsh"));
    }

    #[test]
    fn test_notify_cd_target_writes_to_file() {
        let temp_dir = std::env::temp_dir().join(format!("gwt_test_shell_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let cd_file = temp_dir.join("cd_target.txt");
        unsafe {
            std::env::set_var("GWT_CD_FILE", &cd_file);
        }

        let test_target = Path::new("/some/test/path");
        notify_cd_target(test_target);

        let content = fs::read_to_string(&cd_file).unwrap();
        assert_eq!(content, "/some/test/path");

        unsafe {
            std::env::remove_var("GWT_CD_FILE");
        }
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_notify_cd_target_does_nothing_without_env_var() {
        unsafe {
            std::env::remove_var("GWT_CD_FILE");
        }
        // Should not panic or error
        notify_cd_target(Path::new("/some/other/path"));
    }
}
