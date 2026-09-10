use std::fs;
use std::path::{Path, PathBuf};
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
    fs::write(path.join("README.md"), "initial").unwrap();
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

#[test]
fn test_shell_wrapper_content() {
    assert!(gwt::shell::SHELL_WRAPPER.contains("gwt()"));
    assert!(gwt::shell::SHELL_WRAPPER.contains("GWT_CD_FILE"));
    assert!(gwt::shell::SHELL_WRAPPER.contains("builtin cd"));
}

#[test]
fn test_notify_cd_target_ipc() {
    let temp_dir = std::env::temp_dir().join(format!("gwt_test_ipc_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let cd_file = temp_dir.join("cd_target.txt");
    unsafe {
        std::env::set_var("GWT_CD_FILE", &cd_file);
    }

    let repo_dir = temp_dir.join("myrepo");
    init_git_repo(&repo_dir);

    // Call notify_cd_target
    gwt::shell::notify_cd_target(&repo_dir);

    let recorded = fs::read_to_string(&cd_file).unwrap();
    assert_eq!(recorded, repo_dir.to_string_lossy());

    unsafe {
        std::env::remove_var("GWT_CD_FILE");
    }
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_shell_wrapper_file_matches_constant() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wrapper_path = repo_root.join("_gwt_wrapper");
    assert!(wrapper_path.is_file(), "_gwt_wrapper should exist in repo root");

    let content = fs::read_to_string(wrapper_path).unwrap();
    assert_eq!(content, gwt::shell::SHELL_WRAPPER);
}

#[test]
fn test_shell_wrapper_eval_in_zsh_or_sh() {
    // Check if zsh, sh, or bash can parse the wrapper script without errors
    let shells = ["zsh", "sh", "bash"];
    for sh in &shells {
        let status = Command::new(sh)
            .args(["-n", "-c", gwt::shell::SHELL_WRAPPER])
            .status();
        if let Ok(st) = status {
            assert!(st.success(), "Shell wrapper failed syntax check under {sh}");
        }
    }
}

#[test]
fn test_cli_shell_wrapper_flag() {
    let gwt_bin = env!("CARGO_BIN_EXE_gwt");
    let output = Command::new(gwt_bin)
        .arg("--shell-wrapper")
        .output()
        .expect("failed to execute gwt --shell-wrapper");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, gwt::shell::SHELL_WRAPPER);
}

#[test]
fn test_directory_changing_commands_with_cd_file() {
    let temp_dir = std::env::temp_dir().join(format!("gwt_test_cli_cd_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let repo_dir = temp_dir.join("myrepo");
    init_git_repo(&repo_dir);

    let gwt_bin = env!("CARGO_BIN_EXE_gwt");
    let cd_file = temp_dir.join("gwt_cd_test.txt");

    // 1. Test `gwt add feat1 --ide none`
    let output = Command::new(gwt_bin)
        .current_dir(&repo_dir)
        .env("GWT_CD_FILE", &cd_file)
        .args(["add", "feat1", "--ide", "none"])
        .output()
        .expect("failed to execute gwt add");
    assert!(output.status.success(), "add failed: {:?}", String::from_utf8_lossy(&output.stderr));
    let feat1_path = fs::read_to_string(&cd_file).unwrap();
    assert!(Path::new(&feat1_path).is_dir());
    assert!(feat1_path.contains("feat1"));

    // 2. Test `gwt cd feat1`
    let output = Command::new(gwt_bin)
        .current_dir(&repo_dir)
        .env("GWT_CD_FILE", &cd_file)
        .args(["cd", "feat1"])
        .output()
        .expect("failed to execute gwt cd");
    assert!(output.status.success());
    let recorded_cd = fs::read_to_string(&cd_file).unwrap();
    assert_eq!(recorded_cd, feat1_path);

    // 3. Test `gwt switch feat1 --ide none`
    let output = Command::new(gwt_bin)
        .current_dir(&repo_dir)
        .env("GWT_CD_FILE", &cd_file)
        .args(["switch", "feat1", "--ide", "none"])
        .output()
        .expect("failed to execute gwt switch");
    assert!(output.status.success());
    let recorded_switch = fs::read_to_string(&cd_file).unwrap();
    assert_eq!(recorded_switch, feat1_path);

    // 4. Test `gwt main`
    let output = Command::new(gwt_bin)
        .current_dir(&feat1_path)
        .env("GWT_CD_FILE", &cd_file)
        .args(["main"])
        .output()
        .expect("failed to execute gwt main");
    assert!(output.status.success());
    let recorded_main = fs::read_to_string(&cd_file).unwrap();
    assert_eq!(
        PathBuf::from(recorded_main).canonicalize().unwrap(),
        repo_dir.canonicalize().unwrap()
    );

    // 5. Test `gwt M --ide none`
    let output = Command::new(gwt_bin)
        .current_dir(&feat1_path)
        .env("GWT_CD_FILE", &cd_file)
        .args(["M", "--ide", "none"])
        .output()
        .expect("failed to execute gwt M");
    assert!(output.status.success());
    let recorded_m = fs::read_to_string(&cd_file).unwrap();
    assert_eq!(
        PathBuf::from(recorded_m).canonicalize().unwrap(),
        repo_dir.canonicalize().unwrap()
    );

    // 6. Test `gwt rm` from inside linked worktree
    let output = Command::new(gwt_bin)
        .current_dir(&feat1_path)
        .env("GWT_CD_FILE", &cd_file)
        .args(["rm"])
        .output()
        .expect("failed to execute gwt rm");
    assert!(output.status.success(), "rm failed: {:?}", String::from_utf8_lossy(&output.stderr));
    let recorded_rm = fs::read_to_string(&cd_file).unwrap();
    assert_eq!(
        PathBuf::from(recorded_rm).canonicalize().unwrap(),
        repo_dir.canonicalize().unwrap()
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_shell_wrapper_execution_in_zsh() {
    let temp_dir = std::env::temp_dir().join(format!("gwt_test_zsh_exec_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let repo_dir = temp_dir.join("myrepo");
    init_git_repo(&repo_dir);

    let gwt_bin = env!("CARGO_BIN_EXE_gwt");
    let bin_dir = temp_dir.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let _ = std::os::unix::fs::symlink(gwt_bin, bin_dir.join("gwt"));

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wrapper_file = repo_root.join("_gwt_wrapper");

    let zsh_script = format!(
        r#"
        export PATH="{}:$PATH"
        source "{}"
        cd "{}"
        gwt add feat-zsh --ide none >/dev/null 2>&1
        pwd -P
        "#,
        bin_dir.display(),
        wrapper_file.display(),
        repo_dir.display()
    );

    let output = Command::new("zsh")
        .args(["-c", &zsh_script])
        .output();

    if let Ok(out) = output {
        assert!(out.status.success(), "zsh failed: {:?}", String::from_utf8_lossy(&out.stderr));
        let final_pwd = String::from_utf8_lossy(&out.stdout).trim().to_string();
        assert!(final_pwd.contains("feat-zsh"), "expected final_pwd to contain feat-zsh, got: {}", final_pwd);
    }

    let _ = fs::remove_dir_all(&temp_dir);
}
