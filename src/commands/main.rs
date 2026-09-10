use std::collections::HashSet;
use std::fmt;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use crate::ide::launch_ide;
use crate::repos::{auto_track_repo, get_current_main_repo, is_git_repo, read_tracked_repos};

/// Error types that can occur during the `main` or `M` command.
#[derive(Debug)]
pub enum MainError {
    /// Not inside a git repository and no tracked repository found (exit code 5).
    NoRepoFound,
    /// Interactive prompt: no repository selected (exit code 5).
    NoRepoSelected,
    /// Interactive prompt: invalid selection (exit code 5).
    InvalidSelection,
    /// Invalid argument count for `main` (more than 1 argument provided) (exit code 6).
    InvalidArgCount(String),
    /// Invalid argument count for `M` (more than 1 repository argument provided) (exit code 6).
    InvalidArgCountIde(String),
    /// Multiple exact repository matches found (exit code 7).
    MultipleExactMatches {
        query: String,
        matches: Vec<PathBuf>,
    },
    /// Multiple repository name matches found (exit code 8).
    MultipleNameMatches {
        query: String,
        matches: Vec<PathBuf>,
    },
    /// Multiple repository path matches found (exit code 9).
    MultiplePathMatches {
        query: String,
        matches: Vec<PathBuf>,
    },
    /// No matching repository found for query (exit code 10).
    NoMatch(String),
    /// Missing required argument for `--ide` option (exit code 11).
    MissingIdeArg,
    /// An I/O error occurred (exit code 1).
    Io(io::Error),
}

impl MainError {
    /// Returns the associated process exit code matching `gwt` specifications.
    pub fn exit_code(&self) -> i32 {
        match self {
            MainError::NoRepoFound => 5,
            MainError::NoRepoSelected => 5,
            MainError::InvalidSelection => 5,
            MainError::InvalidArgCount(_) => 6,
            MainError::InvalidArgCountIde(_) => 6,
            MainError::MultipleExactMatches { .. } => 7,
            MainError::MultipleNameMatches { .. } => 8,
            MainError::MultiplePathMatches { .. } => 9,
            MainError::NoMatch(_) => 10,
            MainError::MissingIdeArg => 11,
            MainError::Io(_) => 1,
        }
    }
}

impl fmt::Display for MainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MainError::NoRepoFound => write!(f, "no matching repository found"),
            MainError::NoRepoSelected => write!(f, "no repository selected"),
            MainError::InvalidSelection => write!(f, "invalid selection"),
            MainError::InvalidArgCount(args) => {
                if args.is_empty() {
                    write!(f, "unknown command: 'main '")
                } else {
                    write!(f, "unknown command: 'main {args}'")
                }
            }
            MainError::InvalidArgCountIde(args) => {
                if args.is_empty() {
                    write!(f, "unknown command: 'M '")
                } else {
                    write!(f, "unknown command: 'M {args}'")
                }
            }
            MainError::MultipleExactMatches { query, matches }
            | MainError::MultipleNameMatches { query, matches }
            | MainError::MultiplePathMatches { query, matches } => {
                write!(f, "multiple repositories match '{query}':")?;
                for m in matches {
                    write!(f, "\n  {}", m.display())?;
                }
                Ok(())
            }
            MainError::NoMatch(query) => {
                write!(f, "no matching repository found for '{query}'")
            }
            MainError::MissingIdeArg => write!(f, "--ide requires an argument"),
            MainError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for MainError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MainError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for MainError {
    fn from(err: io::Error) -> Self {
        MainError::Io(err)
    }
}

/// CLI arguments for the `main` / `m` command parsed by `clap`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq, Default)]
pub struct MainArgs {
    /// Repository name or query to match (defaults to main repository of current worktree)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

impl MainArgs {
    /// Creates a new `MainArgs` from a list of arguments.
    pub fn new(args: Vec<String>) -> Self {
        Self { args }
    }

    /// Creates a `MainArgs` from an optional query string.
    pub fn from_query(query: Option<String>) -> Self {
        Self {
            args: query.into_iter().collect(),
        }
    }

    /// Returns the optional query if at most one argument was provided.
    pub fn query(&self) -> Option<&str> {
        self.args.first().map(|s| s.as_str())
    }
}

/// CLI arguments for the `M` / `Main` command parsed by `clap`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq, Default)]
pub struct MainIdeArgs {
    /// Arguments for M command (repository name and optional --ide <IDE>)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

impl MainIdeArgs {
    /// Creates a new `MainIdeArgs` from a list of arguments.
    pub fn new(args: Vec<String>) -> Self {
        Self { args }
    }
}

/// Parsed CLI arguments for the `M` / `Main` command.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedMainIdeArgs {
    /// Target repository name or query
    pub query: Option<String>,
    /// Optional IDE override command
    pub ide: Option<String>,
}

/// Parses CLI arguments for the `M` / `Main` command, handling `--ide <IDE>` / `--ide=<IDE>`.
pub fn parse_main_ide_args(args: &[String]) -> Result<ParsedMainIdeArgs, MainError> {
    let mut override_ide = None;
    let mut positional = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        if arg == "--ide" {
            if i + 1 >= args.len() {
                return Err(MainError::MissingIdeArg);
            }
            override_ide = Some(args[i + 1].clone());
            i += 2;
        } else if let Some(val) = arg.strip_prefix("--ide=") {
            override_ide = Some(val.to_string());
            i += 1;
        } else {
            positional.push(arg.clone());
            i += 1;
        }
    }

    if positional.len() > 1 {
        return Err(MainError::InvalidArgCountIde(positional.join(" ")));
    }

    let query = positional.into_iter().next();

    Ok(ParsedMainIdeArgs {
        query,
        ide: override_ide,
    })
}

/// Resolves the main repository path, optionally reading interactive selection from `prompt_reader`
/// and printing prompts to `prompt_writer`.
pub fn resolve_main_repo_with_io(
    query: Option<&str>,
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
    mut prompt_reader: Option<&mut dyn BufRead>,
    mut prompt_writer: Option<&mut dyn Write>,
) -> Result<PathBuf, MainError> {
    if let Some(q) = query {
        let main_repo = get_current_main_repo(current_dir);
        let mut repo_list = Vec::new();

        if let Some(ref main) = main_repo {
            let _ = auto_track_repo(main, config_dir);
            repo_list.push(main.clone());
        }

        if let Ok(tracked) = read_tracked_repos(config_dir) {
            repo_list.extend(tracked);
        }

        let mut seen = HashSet::new();
        let mut exact_matches = Vec::new();
        let mut matches = Vec::new();
        let mut path_matches = Vec::new();
        let query_lower = q.to_lowercase();

        for repo in repo_list {
            if !repo.is_dir() {
                continue;
            }
            let canon = repo.canonicalize().unwrap_or_else(|_| repo.clone());
            if !seen.insert(canon) {
                continue;
            }
            if !is_git_repo(&repo) {
                continue;
            }

            let repo_name = repo
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let repo_name_lower = repo_name.to_lowercase();
            let repo_path_lower = repo.to_string_lossy().to_lowercase();

            if repo_name_lower == query_lower {
                exact_matches.push(repo.clone());
            }
            if repo_name_lower.contains(&query_lower) {
                matches.push(repo.clone());
            } else if repo_path_lower.contains(&query_lower) {
                path_matches.push(repo.clone());
            }
        }

        if exact_matches.len() == 1 {
            Ok(exact_matches.into_iter().next().unwrap())
        } else if exact_matches.len() > 1 {
            Err(MainError::MultipleExactMatches {
                query: q.to_string(),
                matches: exact_matches,
            })
        } else if matches.len() == 1 {
            Ok(matches.into_iter().next().unwrap())
        } else if matches.len() > 1 {
            Err(MainError::MultipleNameMatches {
                query: q.to_string(),
                matches,
            })
        } else if path_matches.len() == 1 {
            Ok(path_matches.into_iter().next().unwrap())
        } else if path_matches.len() > 1 {
            Err(MainError::MultiplePathMatches {
                query: q.to_string(),
                matches: path_matches,
            })
        } else {
            Err(MainError::NoMatch(q.to_string()))
        }
    } else {
        // No query provided: check if inside git repo
        if let Some(main) = get_current_main_repo(current_dir) {
            let _ = auto_track_repo(&main, config_dir);
            return Ok(main);
        }

        // Outside git repo: check tracked repositories
        let tracked = read_tracked_repos(config_dir).unwrap_or_default();
        let mut options = Vec::new();
        let mut seen = HashSet::new();

        for r in tracked {
            if !r.is_dir() {
                continue;
            }
            let canon = r.canonicalize().unwrap_or_else(|_| r.clone());
            if !seen.insert(canon) {
                continue;
            }
            if !is_git_repo(&r) {
                continue;
            }
            options.push(r);
        }

        if options.is_empty() {
            return Err(MainError::NoRepoFound);
        } else if options.len() == 1 {
            return Ok(options.into_iter().next().unwrap());
        }

        // Multiple tracked repositories: interactively prompt user
        let mut stderr = io::stderr();
        let writer: &mut dyn Write = match prompt_writer.as_mut() {
            Some(w) => *w,
            None => &mut stderr,
        };

        writeln!(writer, "Select a repository to switch to:")?;
        for (i, opt) in options.iter().enumerate() {
            writeln!(writer, "  {}) {}", i + 1, opt.display())?;
        }
        write!(writer, "Enter selection [1-{}]: ", options.len())?;
        writer.flush()?;

        let mut line = String::new();
        let read_ok = match prompt_reader.as_mut() {
            Some(r) => r.read_line(&mut line).is_ok(),
            None => io::stdin().read_line(&mut line).is_ok(),
        };

        if !read_ok {
            return Err(MainError::NoRepoSelected);
        }

        let choice = line.trim();
        if choice.is_empty() {
            return Err(MainError::NoRepoSelected);
        }

        let mut selected = None;
        if let Ok(idx) = choice.parse::<usize>() {
            if idx >= 1 && idx <= options.len() {
                selected = Some(options[idx - 1].clone());
            }
        }

        if selected.is_none() {
            let mut matches = Vec::new();
            let choice_lower = choice.to_lowercase();
            for opt in &options {
                let opt_name = opt
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if opt_name.to_lowercase() == choice_lower
                    || opt.to_string_lossy().to_lowercase() == choice_lower
                {
                    matches.push(opt.clone());
                }
            }

            if matches.len() == 1 {
                selected = Some(matches.into_iter().next().unwrap());
            } else if matches.len() > 1 {
                return Err(MainError::MultipleExactMatches {
                    query: choice.to_string(),
                    matches,
                });
            }
        }

        match selected {
            Some(s) => Ok(s),
            None => Err(MainError::InvalidSelection),
        }
    }
}

/// Resolves the main repository path given an optional query and directories.
pub fn resolve_main_repo(
    query: Option<&str>,
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
) -> Result<PathBuf, MainError> {
    resolve_main_repo_with_io(query, current_dir, config_dir, None, None)
}

/// Executes the `main` command with an argument slice, resolving and printing the repository path.
pub fn main_cmd(
    args: &[String],
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
) -> Result<PathBuf, MainError> {
    if args.len() > 1 {
        return Err(MainError::InvalidArgCount(args.join(" ")));
    }
    let query = args.first().map(|s| s.as_str());
    let path = resolve_main_repo(query, current_dir, config_dir)?;
    println!("{}", path.display());
    crate::shell::notify_cd_target(&path);
    Ok(path)
}

/// Executes the `M` / `Main` command with an argument slice, resolving the repository path,
/// printing it to stdout, and launching the configured IDE.
pub fn main_ide_cmd(
    args: &[String],
    current_dir: Option<&Path>,
    config_dir: Option<&Path>,
    launch: bool,
) -> Result<PathBuf, MainError> {
    let parsed = parse_main_ide_args(args)?;
    let path = resolve_main_repo(parsed.query.as_deref(), current_dir, config_dir)?;
    println!("{}", path.display());
    crate::shell::notify_cd_target(&path);
    if launch {
        launch_ide(parsed.ide.as_deref(), &path, config_dir)?;
    }
    Ok(path)
}

/// Runs the `main` command with parsed `MainArgs`.
pub fn run_args(args: &MainArgs) -> Result<PathBuf, MainError> {
    main_cmd(&args.args, None, None)
}

/// Runs the `M` command with parsed `MainIdeArgs`.
pub fn run_ide_args(args: &MainIdeArgs) -> Result<PathBuf, MainError> {
    main_ide_cmd(&args.args, None, None, true)
}

/// Runs the `main` command with CLI arguments.
pub fn run(args: &[String]) -> Result<PathBuf, MainError> {
    main_cmd(args, None, None)
}

/// Runs the `M` command with CLI arguments.
pub fn run_ide(args: &[String]) -> Result<PathBuf, MainError> {
    main_ide_cmd(args, None, None, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Cursor;
    use std::process::Command;

    fn init_git_repo(path: &Path) {
        fs::create_dir_all(path).unwrap();
        let _ = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["init", "-b", "main"])
            .output();
        let _ = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["config", "user.name", "Test"])
            .output();
        let _ = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["config", "user.email", "test@example.com"])
            .output();
        fs::write(path.join("README.md"), "hello").unwrap();
        let _ = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["add", "."])
            .output();
        let _ = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["commit", "-m", "init"])
            .output();
    }

    fn add_worktree(repo: &Path, branch: &str, wt_path: &Path) {
        let _ = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["worktree", "add", "-b", branch, wt_path.to_str().unwrap()])
            .output();
    }

    #[test]
    fn test_main_in_git_repo_without_args() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_main_in_git_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let config_dir = temp_dir.join("config");
        let result = resolve_main_repo(None, Some(&repo_dir), Some(&config_dir));
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            repo_dir.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_main_in_worktree_without_args() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_main_in_wt_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("myrepo");
        init_git_repo(&repo_dir);

        let wt_path = temp_dir.join("gwt-myrepo").join("feature-test");
        add_worktree(&repo_dir, "feature-test", &wt_path);

        let config_dir = temp_dir.join("config");
        let result = resolve_main_repo(None, Some(&wt_path), Some(&config_dir));
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            repo_dir.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_main_outside_git_repo_no_tracked_repos() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_main_notracked_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let plain_dir = temp_dir.join("plain");
        fs::create_dir_all(&plain_dir).unwrap();

        let config_dir = temp_dir.join("config");
        let result = resolve_main_repo(None, Some(&plain_dir), Some(&config_dir));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 5);
        assert_eq!(err.to_string(), "no matching repository found");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_main_outside_git_repo_single_tracked_repo() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_main_single_tr_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo1");
        init_git_repo(&repo_dir);

        let plain_dir = temp_dir.join("plain");
        fs::create_dir_all(&plain_dir).unwrap();

        let config_dir = temp_dir.join("config");
        auto_track_repo(&repo_dir, Some(&config_dir)).unwrap();

        let result = resolve_main_repo(None, Some(&plain_dir), Some(&config_dir));
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            repo_dir.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_main_outside_git_repo_prompt_selection_number() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_main_prompt_num_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo1 = temp_dir.join("repo1");
        let repo2 = temp_dir.join("repo2");
        init_git_repo(&repo1);
        init_git_repo(&repo2);

        let plain_dir = temp_dir.join("plain");
        fs::create_dir_all(&plain_dir).unwrap();

        let config_dir = temp_dir.join("config");
        auto_track_repo(&repo1, Some(&config_dir)).unwrap();
        auto_track_repo(&repo2, Some(&config_dir)).unwrap();

        // Choice 2
        let mut input = Cursor::new(b"2\n");
        let mut output = Vec::new();
        let result = resolve_main_repo_with_io(
            None,
            Some(&plain_dir),
            Some(&config_dir),
            Some(&mut input),
            Some(&mut output),
        );
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            repo2.canonicalize().unwrap()
        );

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Select a repository to switch to:"));
        assert!(output_str.contains("1)"));
        assert!(output_str.contains("2)"));
        assert!(output_str.contains("Enter selection [1-2]: "));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_main_outside_git_repo_prompt_selection_name() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_main_prompt_name_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo1 = temp_dir.join("alpha");
        let repo2 = temp_dir.join("beta");
        init_git_repo(&repo1);
        init_git_repo(&repo2);

        let plain_dir = temp_dir.join("plain");
        fs::create_dir_all(&plain_dir).unwrap();

        let config_dir = temp_dir.join("config");
        auto_track_repo(&repo1, Some(&config_dir)).unwrap();
        auto_track_repo(&repo2, Some(&config_dir)).unwrap();

        // Select by name "ALPHA" (case-insensitive)
        let mut input = Cursor::new(b"ALPHA\n");
        let mut output = Vec::new();
        let result = resolve_main_repo_with_io(
            None,
            Some(&plain_dir),
            Some(&config_dir),
            Some(&mut input),
            Some(&mut output),
        );
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            repo1.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_main_outside_git_repo_prompt_empty_and_invalid() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_main_prompt_err_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo1 = temp_dir.join("repo1");
        let repo2 = temp_dir.join("repo2");
        init_git_repo(&repo1);
        init_git_repo(&repo2);

        let plain_dir = temp_dir.join("plain");
        fs::create_dir_all(&plain_dir).unwrap();

        let config_dir = temp_dir.join("config");
        auto_track_repo(&repo1, Some(&config_dir)).unwrap();
        auto_track_repo(&repo2, Some(&config_dir)).unwrap();

        // Empty selection
        let mut input_empty = Cursor::new(b"   \n");
        let mut output = Vec::new();
        let res_empty = resolve_main_repo_with_io(
            None,
            Some(&plain_dir),
            Some(&config_dir),
            Some(&mut input_empty),
            Some(&mut output),
        );
        assert!(res_empty.is_err());
        assert_eq!(res_empty.unwrap_err().exit_code(), 5);

        // Invalid selection (out of range index)
        let mut input_invalid_num = Cursor::new(b"99\n");
        let mut output2 = Vec::new();
        let res_invalid = resolve_main_repo_with_io(
            None,
            Some(&plain_dir),
            Some(&config_dir),
            Some(&mut input_invalid_num),
            Some(&mut output2),
        );
        assert!(res_invalid.is_err());
        assert_eq!(res_invalid.unwrap_err().exit_code(), 5);

        // Invalid selection (non-matching name)
        let mut input_invalid_str = Cursor::new(b"nonexistent\n");
        let mut output3 = Vec::new();
        let res_invalid2 = resolve_main_repo_with_io(
            None,
            Some(&plain_dir),
            Some(&config_dir),
            Some(&mut input_invalid_str),
            Some(&mut output3),
        );
        assert!(res_invalid2.is_err());
        assert_eq!(res_invalid2.unwrap_err().exit_code(), 5);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_main_tiered_matching() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_main_tiered_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Repos:
        // 1. my-app
        // 2. my-app-extra
        // 3. something-else (in /path/my-app-dir/something-else)
        let repo1 = temp_dir.join("my-app");
        let repo2 = temp_dir.join("my-app-extra");
        let nested_dir = temp_dir.join("my-app-sub").join("nested");
        init_git_repo(&repo1);
        init_git_repo(&repo2);
        init_git_repo(&nested_dir);

        let config_dir = temp_dir.join("config");
        auto_track_repo(&repo1, Some(&config_dir)).unwrap();
        auto_track_repo(&repo2, Some(&config_dir)).unwrap();
        auto_track_repo(&nested_dir, Some(&config_dir)).unwrap();

        // 1. Exact match "my-app" wins over "my-app-extra" even though "my-app" is also substring
        let r1 = resolve_main_repo(Some("my-app"), None, Some(&config_dir)).unwrap();
        assert_eq!(r1.canonicalize().unwrap(), repo1.canonicalize().unwrap());

        // 2. Substring match on name "extra" matches repo2 only
        let r2 = resolve_main_repo(Some("extra"), None, Some(&config_dir)).unwrap();
        assert_eq!(r2.canonicalize().unwrap(), repo2.canonicalize().unwrap());

        // 3. Substring match on path "nested" matches nested_dir
        let r3 = resolve_main_repo(Some("nested"), None, Some(&config_dir)).unwrap();
        assert_eq!(r3.canonicalize().unwrap(), nested_dir.canonicalize().unwrap());

        // 4. Substring match on path "my-app-sub" matches nested_dir
        let r4 = resolve_main_repo(Some("my-app-sub"), None, Some(&config_dir)).unwrap();
        assert_eq!(r4.canonicalize().unwrap(), nested_dir.canonicalize().unwrap());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_main_disambiguation_errors() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_main_disambig_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo1 = temp_dir.join("dir1").join("common-name");
        let repo2 = temp_dir.join("dir2").join("common-name");
        init_git_repo(&repo1);
        init_git_repo(&repo2);

        let config_dir = temp_dir.join("config");
        auto_track_repo(&repo1, Some(&config_dir)).unwrap();
        auto_track_repo(&repo2, Some(&config_dir)).unwrap();

        // Multiple exact matches -> exit code 7
        let err7 = resolve_main_repo(Some("common-name"), None, Some(&config_dir)).unwrap_err();
        assert_eq!(err7.exit_code(), 7);
        assert!(err7.to_string().contains("multiple repositories match 'common-name':"));

        // Multiple name substring matches -> exit code 8
        let repo_feat1 = temp_dir.join("feat-one");
        let repo_feat2 = temp_dir.join("feat-two");
        init_git_repo(&repo_feat1);
        init_git_repo(&repo_feat2);
        auto_track_repo(&repo_feat1, Some(&config_dir)).unwrap();
        auto_track_repo(&repo_feat2, Some(&config_dir)).unwrap();

        let err8 = resolve_main_repo(Some("feat"), None, Some(&config_dir)).unwrap_err();
        assert_eq!(err8.exit_code(), 8);
        assert!(err8.to_string().contains("multiple repositories match 'feat':"));

        // Multiple path substring matches -> exit code 9
        let nested_a = temp_dir.join("shared-path").join("repo-a");
        let nested_b = temp_dir.join("shared-path").join("repo-b");
        init_git_repo(&nested_a);
        init_git_repo(&nested_b);
        auto_track_repo(&nested_a, Some(&config_dir)).unwrap();
        auto_track_repo(&nested_b, Some(&config_dir)).unwrap();

        let err9 = resolve_main_repo(Some("shared-path"), None, Some(&config_dir)).unwrap_err();
        assert_eq!(err9.exit_code(), 9);
        assert!(err9.to_string().contains("multiple repositories match 'shared-path':"));

        // No match -> exit code 10
        let err10 = resolve_main_repo(Some("nonexistent"), None, Some(&config_dir)).unwrap_err();
        assert_eq!(err10.exit_code(), 10);
        assert_eq!(
            err10.to_string(),
            "no matching repository found for 'nonexistent'"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_main_invalid_arg_count() {
        let err = main_cmd(
            &["arg1".to_string(), "arg2".to_string()],
            None,
            None,
        )
        .unwrap_err();
        assert_eq!(err.exit_code(), 6);
        assert_eq!(err.to_string(), "unknown command: 'main arg1 arg2'");
    }

    #[test]
    fn test_parse_main_ide_args() {
        let parsed = parse_main_ide_args(&["--ide".into(), "code".into(), "my-repo".into()]).unwrap();
        assert_eq!(
            parsed,
            ParsedMainIdeArgs {
                query: Some("my-repo".into()),
                ide: Some("code".into()),
            }
        );

        let parsed2 = parse_main_ide_args(&["--ide=cursor".into()]).unwrap();
        assert_eq!(
            parsed2,
            ParsedMainIdeArgs {
                query: None,
                ide: Some("cursor".into()),
            }
        );

        let parsed3 = parse_main_ide_args(&["my-repo".into(), "--ide=none".into()]).unwrap();
        assert_eq!(
            parsed3,
            ParsedMainIdeArgs {
                query: Some("my-repo".into()),
                ide: Some("none".into()),
            }
        );

        // Missing --ide value -> exit code 11
        let err_missing = parse_main_ide_args(&["--ide".into()]).unwrap_err();
        assert_eq!(err_missing.exit_code(), 11);
        assert_eq!(err_missing.to_string(), "--ide requires an argument");

        // Multiple positional arguments -> exit code 6
        let err_multi = parse_main_ide_args(&["repo1".into(), "repo2".into()]).unwrap_err();
        assert_eq!(err_multi.exit_code(), 6);
        assert_eq!(err_multi.to_string(), "unknown command: 'M repo1 repo2'");
    }

    #[test]
    fn test_main_ide_execution() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_main_ide_exec_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo = temp_dir.join("target-repo");
        init_git_repo(&repo);

        let config_dir = temp_dir.join("config");
        auto_track_repo(&repo, Some(&config_dir)).unwrap();

        let marker_file = repo.join("ide_executed.txt");
        assert!(!marker_file.exists());

        let result = main_ide_cmd(
            &[
                "target-repo".to_string(),
                "--ide".to_string(),
                "touch ide_executed.txt".to_string(),
            ],
            None,
            Some(&config_dir),
            true,
        );
        assert!(result.is_ok());
        assert!(marker_file.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_main_ide_none_does_not_execute() {
        let temp_dir =
            std::env::temp_dir().join(format!("gwt_test_main_ide_none_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo = temp_dir.join("target-repo");
        init_git_repo(&repo);

        let config_dir = temp_dir.join("config");
        auto_track_repo(&repo, Some(&config_dir)).unwrap();

        let result = main_ide_cmd(
            &["target-repo".to_string(), "--ide=none".to_string()],
            None,
            Some(&config_dir),
            true,
        );
        assert!(result.is_ok());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_main_error_exit_codes() {
        assert_eq!(MainError::NoRepoFound.exit_code(), 5);
        assert_eq!(MainError::NoRepoSelected.exit_code(), 5);
        assert_eq!(MainError::InvalidSelection.exit_code(), 5);
        assert_eq!(MainError::InvalidArgCount("".into()).exit_code(), 6);
        assert_eq!(MainError::InvalidArgCountIde("".into()).exit_code(), 6);
        assert_eq!(
            MainError::MultipleExactMatches {
                query: "q".into(),
                matches: vec![]
            }
            .exit_code(),
            7
        );
        assert_eq!(
            MainError::MultipleNameMatches {
                query: "q".into(),
                matches: vec![]
            }
            .exit_code(),
            8
        );
        assert_eq!(
            MainError::MultiplePathMatches {
                query: "q".into(),
                matches: vec![]
            }
            .exit_code(),
            9
        );
        assert_eq!(MainError::NoMatch("q".into()).exit_code(), 10);
        assert_eq!(MainError::MissingIdeArg.exit_code(), 11);
        assert_eq!(
            MainError::Io(io::Error::new(io::ErrorKind::Other, "io error")).exit_code(),
            1
        );
    }
}
