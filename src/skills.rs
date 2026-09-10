use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

/// Supported agent skill environments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkillTarget {
    Agents,
    Opencode,
    Claude,
    Gemini,
}

impl SkillTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillTarget::Agents => "agents",
            SkillTarget::Opencode => "opencode",
            SkillTarget::Claude => "claude",
            SkillTarget::Gemini => "gemini",
        }
    }

    pub fn display_path(&self) -> &'static str {
        match self {
            SkillTarget::Agents => "~/.agents/skills",
            SkillTarget::Opencode => "~/.config/opencode/skills",
            SkillTarget::Claude => "~/.claude/skills",
            SkillTarget::Gemini => "~/.gemini/antigravity-cli/skills, ~/.gemini/config/skills",
        }
    }
}

impl fmt::Display for SkillTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Color codes for terminal output.
#[derive(Debug, Clone, Copy)]
pub struct Colors {
    pub bold: &'static str,
    pub green: &'static str,
    pub blue: &'static str,
    pub yellow: &'static str,
    pub red: &'static str,
    pub dim: &'static str,
    pub reset: &'static str,
}

impl Colors {
    pub fn new(enabled: bool) -> Self {
        if enabled {
            Self {
                bold: "\x1b[1m",
                green: "\x1b[32m",
                blue: "\x1b[34m",
                yellow: "\x1b[33m",
                red: "\x1b[31m",
                dim: "\x1b[2m",
                reset: "\x1b[0m",
            }
        } else {
            Self {
                bold: "",
                green: "",
                blue: "",
                yellow: "",
                red: "",
                dim: "",
                reset: "",
            }
        }
    }
}

/// Returns the default installation directory:
/// `${GWT_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/gwt}`
pub fn get_default_install_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("GWT_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.trim().is_empty() {
            return PathBuf::from(xdg).join("gwt");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".local").join("share").join("gwt")
}

/// Resolves the skills source directory (`.agents/skills`).
pub fn find_skills_src(install_dir: &Path) -> PathBuf {
    let install_skills = install_dir.join(".agents").join("skills");
    if install_skills.is_dir() {
        return install_skills;
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let exe_skills = exe_dir.join(".agents").join("skills");
            if exe_skills.is_dir() {
                return exe_skills;
            }
            if let Some(p1) = exe_dir.parent() {
                let p1_skills = p1.join(".agents").join("skills");
                if p1_skills.is_dir() {
                    return p1_skills;
                }
                if let Some(p2) = p1.parent() {
                    let p2_skills = p2.join(".agents").join("skills");
                    if p2_skills.is_dir() {
                        return p2_skills;
                    }
                }
            }
        }
    }

    let cwd_skills = PathBuf::from(".agents").join("skills");
    if cwd_skills.is_dir() {
        return cwd_skills;
    }

    install_skills
}

/// Formats a path replacing `$HOME/` with `~/` for human-readable output.
pub fn format_display_path(path: &Path) -> String {
    if let Ok(home) = std::env::var("HOME") {
        if let Ok(rel) = path.strip_prefix(&home) {
            return format!("~/{}", rel.display());
        }
    }
    path.display().to_string()
}

/// Returns the destination skill directories for a given target.
pub fn get_target_dirs(target: SkillTarget) -> Vec<PathBuf> {
    match target {
        SkillTarget::Agents => {
            let base = std::env::var("AGENTS_CONFIG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                    PathBuf::from(home).join(".agents")
                });
            vec![base.join("skills")]
        }
        SkillTarget::Opencode => {
            let base = std::env::var("OPENCODE_CONFIG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
                        if !xdg.trim().is_empty() {
                            return PathBuf::from(xdg).join("opencode");
                        }
                    }
                    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                    PathBuf::from(home).join(".config").join("opencode")
                });
            vec![base.join("skills")]
        }
        SkillTarget::Claude => {
            let base = std::env::var("CLAUDE_CONFIG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                    PathBuf::from(home).join(".claude")
                });
            vec![base.join("skills")]
        }
        SkillTarget::Gemini => {
            if let Ok(dir) = std::env::var("GEMINI_SKILLS_DIR") {
                if !dir.trim().is_empty() {
                    return vec![PathBuf::from(dir)];
                }
            }
            if let Ok(dir) = std::env::var("ANTIGRAVITY_SKILLS_DIR") {
                if !dir.trim().is_empty() {
                    return vec![PathBuf::from(dir)];
                }
            }
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let antigravity_dir = std::env::var("ANTIGRAVITY_CONFIG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(&home).join(".gemini").join("antigravity-cli"));
            let gemini_dir = std::env::var("GEMINI_CONFIG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(&home).join(".gemini").join("config"));
            vec![antigravity_dir.join("skills"), gemini_dir.join("skills")]
        }
    }
}

/// Checks if a skill is linked into `dest_dir`.
pub fn is_skill_linked(dest_dir: &Path, skill_src: &Path) -> bool {
    let name = match skill_src.file_name() {
        Some(n) => n,
        None => return false,
    };
    let target = dest_dir.join(name);

    let link_target = match fs::read_link(&target) {
        Ok(p) => p,
        Err(_) => return false,
    };

    if link_target == skill_src {
        return true;
    }

    if let (Ok(target_res), Ok(src_res)) = (fs::canonicalize(&target), fs::canonicalize(skill_src))
    {
        if target_res == src_res {
            return true;
        }
    }

    let link_str = link_target.to_string_lossy();
    let name_str = name.to_string_lossy();
    let pattern1 = format!("/.agents/skills/{name_str}");
    let pattern2 = format!("/.agents/skills/{name_str}/");
    if link_str.ends_with(&pattern1) || link_str.contains(&pattern2) {
        return true;
    }

    false
}

/// Checks if a target environment has any skill linked from `skills_src`.
pub fn is_target_linked(target: SkillTarget, skills_src: &Path) -> bool {
    let dirs = get_target_dirs(target);
    if skills_src.is_dir() {
        if let Ok(entries) = fs::read_dir(skills_src) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    for d in &dirs {
                        if is_skill_linked(d, &entry.path()) {
                            return true;
                        }
                    }
                }
            }
        }
    } else {
        let gwt_src = skills_src.join("gwt");
        for d in &dirs {
            if is_skill_linked(d, &gwt_src) {
                return true;
            }
        }
    }
    false
}

/// Detects which targets are currently linked. Returns list and comma-separated string.
pub fn detect_linked_targets(skills_src: &Path) -> (Vec<SkillTarget>, String) {
    let mut detected = Vec::new();
    let mut names = Vec::new();
    for &target in &[
        SkillTarget::Agents,
        SkillTarget::Opencode,
        SkillTarget::Claude,
        SkillTarget::Gemini,
    ] {
        if is_target_linked(target, skills_src) {
            detected.push(target);
            names.push(target.as_str());
        }
    }
    let s = names.join(", ");
    (detected, s)
}

/// Parses selection input string into set of selected targets and warnings.
pub fn parse_skills_selection(input: &str) -> (HashSet<SkillTarget>, Vec<String>) {
    let mut selected = HashSet::new();
    let mut warnings = Vec::new();

    let cleaned = input.replace([',', ';'], " ");
    for token in cleaned.split_whitespace() {
        let lower = token.to_lowercase();
        match lower.as_str() {
            "1" | "agents" | "agent" => {
                selected.insert(SkillTarget::Agents);
            }
            "2" | "opencode" => {
                selected.insert(SkillTarget::Opencode);
            }
            "3" | "claude" => {
                selected.insert(SkillTarget::Claude);
            }
            "4" | "gemini" | "antigravity" | "agy" | "yourself" => {
                selected.insert(SkillTarget::Gemini);
            }
            "5" | "all" | "1-4" => {
                selected.insert(SkillTarget::Agents);
                selected.insert(SkillTarget::Opencode);
                selected.insert(SkillTarget::Claude);
                selected.insert(SkillTarget::Gemini);
            }
            "0" | "6" | "none" | "no" | "n" | "skip" => {
                selected.clear();
                return (selected, warnings);
            }
            _ => {
                warnings.push(format!("Unknown skill target: '{token}'"));
            }
        }
    }

    (selected, warnings)
}

fn create_symlink(src: &Path, target: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, target)
    }
    #[cfg(windows)]
    {
        if src.is_dir() {
            std::os::windows::fs::symlink_dir(src, target)
        } else {
            std::os::windows::fs::symlink_file(src, target)
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(io::Error::new(io::ErrorKind::Unsupported, "Symlinks not supported on this platform"))
    }
}

fn remove_symlink(target: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        fs::remove_file(target).or_else(|_| fs::remove_dir(target))
    }
    #[cfg(not(windows))]
    {
        fs::remove_file(target)
    }
}

/// Symlinks a single skill into `dest_dir`.
pub fn symlink_skill<W: Write>(
    src: &Path,
    dest_dir: &Path,
    dry_run: bool,
    colors: &Colors,
    out: &mut W,
) -> io::Result<()> {
    let name = match src.file_name() {
        Some(n) => n,
        None => return Ok(()),
    };
    let target = dest_dir.join(name);
    let disp_target = format_display_path(&target);
    let disp_src = format_display_path(src);

    if !dry_run {
        let _ = fs::create_dir_all(dest_dir);
    }

    if target.is_symlink() {
        if let Ok(current) = fs::read_link(&target) {
            if current == src {
                writeln!(
                    out,
                    "{}==>{} {}Already linked: {} -> {}{}",
                    colors.blue, colors.reset, colors.bold, disp_target, disp_src, colors.reset
                )?;
                return Ok(());
            }
        }
        if dry_run {
            writeln!(
                out,
                "{}==>{} {}Would update symlink: {} -> {}{}",
                colors.blue, colors.reset, colors.bold, disp_target, disp_src, colors.reset
            )?;
        } else {
            let _ = remove_symlink(&target);
            create_symlink(src, &target)?;
            writeln!(
                out,
                "{}==>{} {}Updated symlink: {} -> {}{}",
                colors.green, colors.reset, colors.bold, disp_target, disp_src, colors.reset
            )?;
        }
    } else if target.exists() {
        writeln!(
            out,
            "{}warning:{} {} exists and is not a symlink; skipping",
            colors.yellow, colors.reset, disp_target
        )?;
    } else if dry_run {
        writeln!(
            out,
            "{}==>{} {}Would symlink: {} -> {}{}",
            colors.blue, colors.reset, colors.bold, disp_target, disp_src, colors.reset
        )?;
    } else {
        create_symlink(src, &target)?;
        writeln!(
            out,
            "{}==>{} {}Symlinked: {} -> {}{}",
            colors.green, colors.reset, colors.bold, disp_target, disp_src, colors.reset
        )?;
    }

    Ok(())
}

/// Unlinks a skill from `dest_dir`.
pub fn unlink_skill<W: Write>(
    src: &Path,
    dest_dir: &Path,
    dry_run: bool,
    colors: &Colors,
    out: &mut W,
) -> io::Result<()> {
    let name = match src.file_name() {
        Some(n) => n,
        None => return Ok(()),
    };
    let target = dest_dir.join(name);
    let disp_target = format_display_path(&target);

    if is_skill_linked(dest_dir, src) {
        if dry_run {
            writeln!(
                out,
                "{}==>{} {}Would remove symlink: {}{}",
                colors.blue, colors.reset, colors.bold, disp_target, colors.reset
            )?;
        } else {
            let _ = remove_symlink(&target);
            writeln!(
                out,
                "{}==>{} {}Removed symlink: {}{}",
                colors.green, colors.reset, colors.bold, disp_target, colors.reset
            )?;
        }
    }
    Ok(())
}

/// Symlinks all skills in `skills_src` into `dest_dir`.
pub fn link_dir_skills<W: Write>(
    dest_dir: &Path,
    skills_src: &Path,
    dry_run: bool,
    colors: &Colors,
    out: &mut W,
) -> io::Result<()> {
    if skills_src.is_dir() {
        if let Ok(entries) = fs::read_dir(skills_src) {
            let mut paths: Vec<_> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            paths.sort();
            for skill in paths {
                symlink_skill(&skill, dest_dir, dry_run, colors, out)?;
            }
        }
    } else if dry_run {
        symlink_skill(&skills_src.join("gwt"), dest_dir, dry_run, colors, out)?;
    }
    Ok(())
}

/// Unlinks all skills in `skills_src` from `dest_dir`.
pub fn unlink_dir_skills<W: Write>(
    dest_dir: &Path,
    skills_src: &Path,
    dry_run: bool,
    colors: &Colors,
    out: &mut W,
) -> io::Result<()> {
    if skills_src.is_dir() {
        if let Ok(entries) = fs::read_dir(skills_src) {
            let mut paths: Vec<_> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            paths.sort();
            for skill in paths {
                unlink_skill(&skill, dest_dir, dry_run, colors, out)?;
            }
        }
    } else if dest_dir.is_dir() {
        unlink_skill(&skills_src.join("gwt"), dest_dir, dry_run, colors, out)?;
    }
    Ok(())
}

/// Applies selection to a single target environment.
pub fn apply_target_selection<W: Write>(
    target: SkillTarget,
    should_link: bool,
    skills_src: &Path,
    dry_run: bool,
    colors: &Colors,
    out: &mut W,
) -> io::Result<()> {
    let dirs = get_target_dirs(target);
    for dir in dirs {
        if should_link {
            link_dir_skills(&dir, skills_src, dry_run, colors, out)?;
        } else {
            unlink_dir_skills(&dir, skills_src, dry_run, colors, out)?;
        }
    }
    Ok(())
}

/// Applies selection to all target environments.
pub fn apply_all_targets<W: Write>(
    selected: &HashSet<SkillTarget>,
    skills_src: &Path,
    dry_run: bool,
    colors: &Colors,
    out: &mut W,
) -> io::Result<()> {
    if !skills_src.is_dir() && !dry_run {
        writeln!(
            out,
            "{}warning:{} Skills directory not found at {}; skipping skill symlinking.",
            colors.yellow,
            colors.reset,
            skills_src.display()
        )?;
        return Ok(());
    }

    let (_, detected_targets) = detect_linked_targets(skills_src);
    let has_selected = !selected.is_empty();

    if !has_selected && detected_targets.is_empty() {
        writeln!(
            out,
            "{}==>{} {}No agent skills selected or currently linked.{}",
            colors.blue, colors.reset, colors.bold, colors.reset
        )?;
        return Ok(());
    }

    if !has_selected {
        writeln!(
            out,
            "{}==>{} {}Unlinking agent skills...{}",
            colors.blue, colors.reset, colors.bold, colors.reset
        )?;
    } else {
        writeln!(out)?;
        writeln!(
            out,
            "{}==>{} {}Symlinking skills...{}",
            colors.blue, colors.reset, colors.bold, colors.reset
        )?;
    }

    for &target in &[
        SkillTarget::Agents,
        SkillTarget::Opencode,
        SkillTarget::Claude,
        SkillTarget::Gemini,
    ] {
        apply_target_selection(
            target,
            selected.contains(&target),
            skills_src,
            dry_run,
            colors,
            out,
        )?;
    }

    Ok(())
}

/// Updates symlinks for currently linked agents (`gwt skills sync`).
pub fn cmd_sync<W: Write>(
    skills_src: &Path,
    dry_run: bool,
    colors: &Colors,
    out: &mut W,
) -> io::Result<()> {
    let (detected, detected_targets) = detect_linked_targets(skills_src);
    if detected_targets.is_empty() {
        writeln!(
            out,
            "{}==>{} {}No linked agent skills detected.{}",
            colors.blue, colors.reset, colors.bold, colors.reset
        )?;
        return Ok(());
    }

    if !skills_src.is_dir() && !dry_run {
        writeln!(
            out,
            "{}warning:{} Skills directory not found at {}; skipping skill symlinking.",
            colors.yellow,
            colors.reset,
            skills_src.display()
        )?;
        return Ok(());
    }

    writeln!(
        out,
        "{}==>{} {}Detected linked agent skills: {}{}",
        colors.blue, colors.reset, colors.bold, detected_targets, colors.reset
    )?;
    writeln!(
        out,
        "{}==>{} {}Updating skills symlinks...{}",
        colors.blue, colors.reset, colors.bold, colors.reset
    )?;

    for &target in &[
        SkillTarget::Agents,
        SkillTarget::Opencode,
        SkillTarget::Claude,
        SkillTarget::Gemini,
    ] {
        if detected.contains(&target) {
            apply_target_selection(target, true, skills_src, dry_run, colors, out)?;
        }
    }

    Ok(())
}

/// Lists current symlink status for all agents (`gwt skills list`).
pub fn cmd_list<W: Write>(skills_src: &Path, colors: &Colors, out: &mut W) -> io::Result<()> {
    let (detected, _) = detect_linked_targets(skills_src);
    writeln!(out, "Agent skill symlink status:")?;

    let items = [
        (SkillTarget::Agents, "agents", "~/.agents/skills"),
        (SkillTarget::Opencode, "opencode", "~/.config/opencode/skills"),
        (SkillTarget::Claude, "claude", "~/.claude/skills"),
        (
            SkillTarget::Gemini,
            "gemini",
            "~/.gemini/antigravity-cli/skills, ~/.gemini/config/skills",
        ),
    ];

    for (target, name, disp) in items {
        if detected.contains(&target) {
            writeln!(
                out,
                "  {:<10} {}[linked]{}     ({})",
                name, colors.green, colors.reset, disp
            )?;
        } else {
            writeln!(
                out,
                "  {:<10} {}[not linked]{} ({})",
                name, colors.dim, colors.reset, disp
            )?;
        }
    }

    Ok(())
}

/// Interactive selection prompt (`gwt skills prompt`).
pub fn cmd_prompt<R: BufRead, W: Write>(
    skills_src: &Path,
    dry_run: bool,
    colors: &Colors,
    prompt_reader: Option<&mut R>,
    out: &mut W,
) -> io::Result<()> {
    let (detected, detected_targets) = detect_linked_targets(skills_src);

    let status_tag = |target: SkillTarget| -> String {
        if detected.contains(&target) {
            format!(" {}[linked]{}", colors.green, colors.reset)
        } else {
            String::new()
        }
    };

    writeln!(out)?;
    writeln!(
        out,
        "{}==>{} {}Symlink skills to global agent directories?{}",
        colors.blue, colors.reset, colors.bold, colors.reset
    )?;
    writeln!(
        out,
        "Select targets to link skills (comma-separated or numbers):"
    )?;
    writeln!(
        out,
        "  1) agents       (~/.agents/skills){}",
        status_tag(SkillTarget::Agents)
    )?;
    writeln!(
        out,
        "  2) opencode     (~/.config/opencode/skills){}",
        status_tag(SkillTarget::Opencode)
    )?;
    writeln!(
        out,
        "  3) claude       (~/.claude/skills){}",
        status_tag(SkillTarget::Claude)
    )?;
    writeln!(
        out,
        "  4) gemini       (~/.gemini/antigravity-cli/skills, ~/.gemini/config/skills){}",
        status_tag(SkillTarget::Gemini)
    )?;
    writeln!(out, "  5) all")?;
    writeln!(out, "  6) none")?;
    writeln!(out)?;

    let default_prompt = if !detected_targets.is_empty() {
        detected_targets.clone()
    } else {
        "none".to_string()
    };

    let user_input = match prompt_reader {
        Some(reader) => {
            write!(out, "Enter choice(s) [default: {default_prompt}]: ")?;
            out.flush()?;
            let mut line = String::new();
            let _ = reader.read_line(&mut line);
            line
        }
        None => {
            let mut read_line = String::new();
            let mut tty_read = false;
            #[cfg(unix)]
            {
                if let Ok(mut tty) = fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open("/dev/tty")
                {
                    let _ = write!(tty, "Enter choice(s) [default: {default_prompt}]: ");
                    let _ = tty.flush();
                    let mut reader = io::BufReader::new(&tty);
                    if reader.read_line(&mut read_line).is_ok() {
                        tty_read = true;
                    }
                }
            }
            if !tty_read {
                if io::stdin().is_terminal() {
                    write!(out, "Enter choice(s) [default: {default_prompt}]: ")?;
                    out.flush()?;
                    let mut line = String::new();
                    let _ = io::stdin().read_line(&mut line);
                    line
                } else {
                    default_prompt.clone()
                }
            } else {
                read_line
            }
        }
    };

    let trimmed = user_input.trim();
    let choice = if trimmed.is_empty() {
        &default_prompt
    } else {
        trimmed
    };

    let (selected, warnings) = parse_skills_selection(choice);
    for warn in warnings {
        writeln!(out, "{}warning:{} {}", colors.yellow, colors.reset, warn)?;
    }

    apply_all_targets(&selected, skills_src, dry_run, colors, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_skills_selection_individual() {
        let (sel, warns) = parse_skills_selection("agents");
        assert_eq!(sel, HashSet::from([SkillTarget::Agents]));
        assert!(warns.is_empty());

        let (sel, warns) = parse_skills_selection("1,3");
        assert_eq!(sel, HashSet::from([SkillTarget::Agents, SkillTarget::Claude]));
        assert!(warns.is_empty());

        let (sel, warns) = parse_skills_selection("claude, gemini");
        assert_eq!(sel, HashSet::from([SkillTarget::Claude, SkillTarget::Gemini]));
        assert!(warns.is_empty());
    }

    #[test]
    fn test_parse_skills_selection_all_none() {
        let (sel, warns) = parse_skills_selection("all");
        assert_eq!(sel.len(), 4);
        assert!(warns.is_empty());

        let (sel, warns) = parse_skills_selection("none");
        assert!(sel.is_empty());
        assert!(warns.is_empty());

        let (sel, warns) = parse_skills_selection("all, none");
        assert!(sel.is_empty());
        assert!(warns.is_empty());
    }

    #[test]
    fn test_parse_skills_selection_unknown_token() {
        let (sel, warns) = parse_skills_selection("claude, unknown_agent");
        assert_eq!(sel, HashSet::from([SkillTarget::Claude]));
        assert_eq!(warns.len(), 1);
        assert!(warns[0].contains("unknown_agent"));
    }

    #[test]
    fn test_format_display_path() {
        if let Ok(home) = std::env::var("HOME") {
            let path = PathBuf::from(&home).join(".agents").join("skills");
            assert_eq!(format_display_path(&path), "~/.agents/skills");
        }
    }

    #[test]
    fn test_cmd_list_output() {
        let colors = Colors::new(false);
        let mut out = Vec::new();
        let dummy_src = PathBuf::from("/nonexistent/skills");
        cmd_list(&dummy_src, &colors, &mut out).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("Agent skill symlink status:"));
        assert!(output.contains("agents"));
        assert!(output.contains("opencode"));
        assert!(output.contains("claude"));
        assert!(output.contains("gemini"));
    }

    #[test]
    fn test_cmd_prompt_with_reader() {
        let colors = Colors::new(false);
        let mut out = Vec::new();
        let dummy_src = PathBuf::from("/nonexistent/skills");
        let mut input = Cursor::new(b"none\n");
        cmd_prompt(&dummy_src, true, &colors, Some(&mut input), &mut out).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("Symlink skills to global agent directories?"));
    }

    #[test]
    fn test_symlink_and_unlink_skill() {
        let temp_dir = std::env::temp_dir().join(format!("gwt_test_skills_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let src_dir = temp_dir.join("src").join("gwt");
        let dest_dir = temp_dir.join("dest");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dest_dir).unwrap();

        let colors = Colors::new(false);
        let mut out = Vec::new();

        // 1. Symlink
        symlink_skill(&src_dir, &dest_dir, false, &colors, &mut out).unwrap();
        assert!(dest_dir.join("gwt").is_symlink());
        assert!(is_skill_linked(&dest_dir, &src_dir));

        // 2. Symlink again (already linked)
        out.clear();
        symlink_skill(&src_dir, &dest_dir, false, &colors, &mut out).unwrap();
        let s = String::from_utf8(out.clone()).unwrap();
        assert!(s.contains("already linked"));

        // 3. Unlink
        out.clear();
        unlink_skill(&src_dir, &dest_dir, false, &colors, &mut out).unwrap();
        assert!(!dest_dir.join("gwt").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
