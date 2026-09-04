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

Whenever executing commands that launch an IDE by default (`add`, `pull`, `switch`, `M`), you **must** supply `--ide none` or set `GWT_IDE=none`.

```bash
# Preferred CLI flag:
gwt add --ide none <branch-name>

# Or via environment variable:
GWT_IDE=none gwt add <branch-name>
```

Similarly, commands that launch an agent (`agent`, `a`, or `add` with `--agent` / `-a`) can be suppressed using `--agent none`, `-a none`, or the `GWT_AGENT=none` environment variable:

```bash
gwt a --agent none <worktree-name>
# or
GWT_AGENT=none gwt a <worktree-name>
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

# Launch configured agent instead of IDE:
gwt add --agent <branch-name>
# or shorthand:
gwt add -a <branch-name>

# Launch a specific named agent instead of IDE:
gwt add --agent cursor <branch-name>
gwt add -a cursor <branch-name>

# Suppress agent launch when using agent mode:
gwt add -a none <branch-name>
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

### 4. Switch to an Existing Worktree with Agent
Changes directory to a worktree matching the specified query name and launches the configured agent. If no agent is configured, prompts to enter the launch command on first run.

```bash
# Option A: Launch configured agent
gwt agent <worktree-name>
# or shorthand:
gwt a <worktree-name>

# Option B: Suppress agent launch
gwt agent --agent none <worktree-name>
# or shorthand:
gwt a --agent none <worktree-name>
```

### 5. Switch to the Main Repository
Changes directory to the main repository for the current worktree, or searches tracked repositories if a name is provided.
If not in a repository and no name is provided, prompts to choose from tracked repositories (or switches automatically if only one exists).

```bash
# Option A: Return to main repository without launching an IDE:
gwt main
# or shorthand:
gwt m

# Option B: Switch to main repository and launch IDE (or suppress IDE):
# Do not run this yourself, you can use the above `m` command instead.
gwt M
gwt M --ide none

# Switch to a specific main repository by name:
gwt main <repo-name>
gwt m <repo-name>
gwt M --ide none <repo-name>
```

### 6. List Tracked Worktrees
Displays worktrees across tracked repositories, optionally filtered by repository name or query. Extra arguments (such as `--porcelain`) are passed through to `git worktree list`.

```bash
# List all tracked worktrees:
gwt ls
# or:
gwt list

# Filter to worktrees for a specific repository (exact or partial name match):
gwt ls <repo-name>
gwt list <repo-name>

# Pass git worktree list flags (e.g. porcelain output):
gwt ls --porcelain
gwt ls <repo-name> --porcelain
```

### 7. Remove a Worktree
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

### 8. Track a Git Repository
Adds the current repository (or a specified repository path) to the list of tracked repositories.

```bash
# Track the current repository:
gwt track
# or shorthand:
gwt t

# Track a specific repository path:
gwt track /path/to/repo
gwt t /path/to/repo
```

### 9. Configuration Management
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

### 10. Upgrade `gwt`
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

---

## Exit Codes

The `gwt` utility returns `0` on success and unique non-zero exit codes for error handling in scripts:

| Exit Code | Command / Category | Description |
|-----------|--------------------|-------------|
| `0` | Success | Command completed successfully. |
| `1` | `cd` | Invalid argument count (expected exactly 1 argument). |
| `2` | `cd` | Multiple matching worktrees found in current repository. |
| `3` | `cd` | Multiple matching worktrees found across tracked repositories. |
| `4` | `cd` | No matching worktree found for query. |
| `5` | `main` | Not inside a git repository and no repository specified or found. |
| `6` | `main` | Invalid argument count (more than 1 argument provided). |
| `7` | `main` | Multiple exact repository matches found. |
| `8` | `main` | Multiple repository name matches found. |
| `9` | `main` | Multiple repository path matches found. |
| `10` | `main` | No matching repository found for query. |
| `11` | `switch` | Missing required argument for `--ide` option. |
| `12` | `switch` | Invalid argument count (expected exactly 1 worktree name). |
| `13` | `remove` | Cannot remove main repository (a worktree target must be specified). |
| `14` | `remove` | Failed to change directory to main repository before removal. |
| `15` | `remove` | `git worktree remove` command failed. |
| `16` | `pull` | Missing required argument for `--ide` option. |
| `17` | `pull` | Invalid argument count (expected exactly 1 branch name). |
| `18` | `pull` | Failed to determine target worktree directory location. |
| `19` | `pull` | Failed to create worktree parent directory. |
| `20` | `pull` | `git fetch origin` command failed. |
| `21` | `pull` | `git worktree add` command failed. |
| `22` | `pull` | Failed to change directory to newly created worktree. |
| `23` | `add` | Missing required argument for `--ide` option. |
| `24` | `add` | Invalid argument count (expected exactly 1 branch name). |
| `25` | `add` | Failed to determine target worktree directory location. |
| `26` | `add` | Failed to create worktree parent directory. |
| `27` | `add` | `git worktree add` command failed. |
| `28` | `add` | Failed to change directory to newly created worktree. |
| `29` | `config` | Invalid argument count for `gwt config get` (expected exactly 1 key). |
| `30` | `config` | Specified configuration key not found in `gwt config get`. |
| `31` | `config` | Invalid argument count for `gwt config set` (expected key and value). |
| `32` | `config` | Invalid argument count for `gwt config unset` (expected exactly 1 key). |
| `33` | `config` | Specified configuration key not found in `gwt config <key>`. |
| `34` | `upgrade` | `gwt` repository not found at target directory. |
| `35` | `upgrade` | `git pull` command failed during upgrade. |
| `36` | `track` | Not inside a git repository and no repository specified. |
| `37` | `track` | Invalid argument count (more than 1 argument provided). |
| `38` | `track` | Specified path is not a git repository. |
| `39` | `list` | Invalid argument count (more than 1 repository specified). |
| `40` | `list` | Multiple exact repository matches found. |
| `41` | `list` | Multiple repository name matches found. |
| `42` | `list` | Multiple repository path matches found. |
| `43` | `list` | No matching repository found for query. |
| `44` | `agent` | Missing required argument for `--agent` option. |
| `45` | `agent` | Invalid argument count (expected exactly 1 worktree name). |
| `46` | `agent` | No agent configured. |
