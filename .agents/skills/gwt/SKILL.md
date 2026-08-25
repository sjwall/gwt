---
name: gwt
description: >-
  Manage git worktrees using the gwt helper tool without launching an interactive IDE or editor. Use this skill whenever creating, pulling, switching, listing, or removing git worktrees in repositories managed by gwt.
---

# `gwt` Worktree Helper Skill

The `gwt` tool simplifies git worktree creation, directory switching, and dependency installation. By default, `gwt` attempts to launch an interactive editor/IDE (such as `nvim` or VS Code) upon creating or switching worktrees.

When running as an automated agent, **always suppress the IDE launch** using `--ide none` or the `GWT_IDE=none` environment variable to prevent commands from hanging or blocking waiting for terminal input.

---

## Key Rule: Headless Execution

Whenever executing commands that launch an IDE by default (`add`, `pull`, `switch`), you **must** supply `--ide none` or set `GWT_IDE=none`.

```bash
# Preferred CLI flag:
gwt add --ide none <branch-name>

# Or via environment variable:
GWT_IDE=none gwt add <branch-name>
```

---

## Command Reference for Agents

### 1. Create a New Worktree Branch
Creates a new worktree under `../gwt-<repo>/<branch-name>`, installs dependencies if `yarn.lock` is present, and skips IDE launch.

```bash
gwt add --ide none <branch-name>
# or shorthand:
gwt --ide none <branch-name>
```

### 2. Pull a Remote Branch into a Worktree
Fetches `origin/<branch-name>`, creates a tracking worktree at `../gwt-<repo>/<branch-name>`, installs dependencies, and skips IDE launch.

```bash
gwt pull --ide none <branch-name>
# or shorthand:
gwt p --ide none <branch-name>
```

### 3. Switch to an Existing Worktree
Changes directory to a worktree matching the specified query name.

```bash
# Option A: gwt cd (changes directory without launching an IDE)
gwt cd <worktree-name>

# Option B: gwt switch with IDE suppressed
gwt switch --ide none <worktree-name>
# or shorthand:
gwt s --ide none <worktree-name>
```

### 4. List All Tracked Worktrees
Displays all worktrees across tracked repositories.

```bash
gwt ls
# or:
gwt list
```

### 5. Remove a Worktree
Deletes the specified worktree or the current worktree if run from within one (automatically changes directory to the main repository before removal).

```bash
# When inside a worktree, remove the current worktree:
gwt rm

# Remove a specific worktree by name:
gwt rm <worktree-name>

# Force removal if worktree has uncommitted changes or untracked files:
gwt rm -f
gwt rm -f <worktree-name>
```

### 6. Persistent Configuration
To permanently disable IDE launching for the current user configuration:

```bash
gwt ide none
# or:
gwt config set ide none
```

To inspect the current configuration:

```bash
gwt config
```

---

## Running `gwt` in Scripts & Subshells

`gwt` is implemented as a Zsh function. If executing from non-interactive shells or script runners:

1. **Ensure `gwt.sh` is sourced**:
   ```zsh
   source "${XDG_DATA_HOME:-$HOME/.local/share}/gwt/gwt.sh"
   ```
2. **Execute in Zsh**:
   ```zsh
   zsh -c 'source "${XDG_DATA_HOME:-$HOME/.local/share}/gwt/gwt.sh" && gwt add --ide none <branch-name>'
   ```
