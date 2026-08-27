# gwt (Git WorkTree Helper)

A lightweight CLI tool to simplify creating, switching, and managing Git worktrees across repositories.

## Codebase Overview

- **`./gwt.sh`**: Main implementation file containing the `gwt` Zsh function and embedded Zsh autocompletion logic.
- **`./_gwt`**: Standalone Zsh autocompletion file (`#compdef gwt`) for autoloading and plugin managers.
- **`./install.sh`**: POSIX shell installation and upgrade script.
- **`./README.adoc`**: Project documentation, command reference, configuration paths, and exit codes.
- **`./agents/skills/gwt`**: Skill for agents using this tool.

## Key Conventions & Agent Guidelines

- **Headless Execution**: `gwt` launches an IDE by default (`nvim`, `code`, etc.) on commands like `add`, `pull`, and `switch`. Automated agents and non-interactive scripts must suppress IDE launch using `--ide none` or `GWT_IDE=none`.
- **Shell Environment**: `gwt.sh` is written specifically for **Zsh** and uses Zsh syntax and parameter expansion.

