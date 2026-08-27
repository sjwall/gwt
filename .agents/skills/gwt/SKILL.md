---
name: gwt
description: >-
  Manage git worktrees and repositories using the gwt helper tool without launching an interactive IDE or editor. Use this skill whenever creating, pulling, switching, listing, or removing git worktrees in repositories managed by gwt.
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
Creates a new worktree under `../gwt-<repo>/<branch-name>`, installs dependencies if `yarn.lock` is present (unless `--no-install` is supplied), and skips IDE launch.

```bash
gwt add --ide none <branch-name>
# or shorthand:
gwt --ide none <branch-name>

# Skip running yarn / dependency installation:
gwt add --ide none --no-install <branch-name>
```

### 2. Pull a Remote Branch into a Worktree
Fetches `origin/<branch-name>`, creates a tracking worktree at `../gwt-<repo>/<branch-name>`, installs dependencies (unless `--no-install` is supplied), and skips IDE launch.

```bash
gwt pull --ide none <branch-name>
# or shorthand:
gwt p --ide none <branch-name>

# Skip running yarn / dependency installation:
gwt pull --ide none --no-install <branch-name>
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

### 4. Switch to the Main Repository
Changes directory to the main repository for the current worktree, or searches tracked repositories if a name is provided.

```bash
# Return to the main repository for the current worktree:
gwt main
# or shorthand:
gwt m

# Switch to a specific main repository by name:
gwt main <repo-name>
gwt m <repo-name>
```

### 5. List All Tracked Worktrees
Displays all worktrees across tracked repositories. Extra arguments are passed through to `git worktree list`.

```bash
gwt ls
# or:
gwt list

# Pass git worktree list flags (e.g. porcelain output):
gwt ls --porcelain
```

### 6. Remove a Worktree
Deletes the specified worktree or the current worktree if run from within one (automatically changes directory to the main repository before removal).

```bash
# When inside a worktree, remove the current worktree:
gwt rm
# or:
gwt remove

# Remove a specific worktree by name:
gwt rm <worktree-name>

# Force removal if worktree has uncommitted changes or untracked files:
gwt rm -f
gwt rm -f <worktree-name>
# (--force and -force are also supported)
gwt rm --force <worktree-name>
```

### 7. Configuration Management
Inspect, set, or remove configuration options:

```bash
# Permanently disable IDE launching for the current user configuration:
gwt ide none
# or:
gwt config set ide none

# Inspect all current configuration settings:
gwt config

# Get or set a specific config key:
gwt config get <key>
gwt config set <key> <value>

# Unset/remove a config key:
gwt config unset <key>
# or shorthand:
gwt config rm <key>
```

### 8. Upgrade `gwt`
Upgrades the `gwt` repository via `git pull` and re-sources `gwt.sh`.

```bash
gwt upgrade
```

---

## Configuration & Storage Locations

* **Tracked repositories**: `${XDG_CONFIG_HOME:-~/.config}/gwt/repos`
* **Custom worktree parent locations**: `${XDG_CONFIG_HOME:-~/.config}/gwt/locations`
* **Configuration settings**: `${XDG_CONFIG_HOME:-~/.config}/gwt/config`

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
