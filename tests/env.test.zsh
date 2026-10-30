#!/usr/bin/env zsh
# Tests for `gwt env` (commands/env.sh and commands/env-*.sh).
#
# Environment files are tracked one path per line (relative to the main repo)
# in a plain-text config file. Resolution order for that config file:
#   1. $XDG_CONFIG_HOME/gwt/envs/<repo-name>  (user level, only with -u for
#      add/remove; always wins for list/sync)
#   2. $PWD/.gwt/envs                         (worktree-local, overrides main)
#   3. $main_repo/.gwt/envs                   (fallback)
#
# Exit code contract:
#   0  - success (also for "nothing to remove", missing tracked files, and
#        `env sync <path>` despite its broken source override)
#   58 - not inside a git repository (all subcommands)
#   61 - missing or unknown subcommand

# ---------------------------------------------------------------------------
# Exit code 58: not inside a git repository
# ---------------------------------------------------------------------------

test_env_without_subcommand_outside_repo_returns_58() {
  cd "$SANDBOX/work"

  invoke_gwt env

  assert_status 58
  assert_contains "$run_stderr" "gwt: not inside a git repository"
  assert_stdout_empty
}

test_env_add_outside_repo_returns_58() {
  cd "$SANDBOX/work"

  invoke_gwt env add .env

  assert_status 58
  assert_contains "$run_stderr" "gwt: not inside a git repository"
  assert_stdout_empty
}

test_env_list_and_ls_outside_repo_returns_58() {
  cd "$SANDBOX/work"

  invoke_gwt env list

  assert_status 58
  assert_contains "$run_stderr" "gwt: not inside a git repository"

  invoke_gwt env ls

  assert_status 58
  assert_contains "$run_stderr" "gwt: not inside a git repository"
}

test_env_sync_outside_repo_returns_58() {
  cd "$SANDBOX/work"

  invoke_gwt env sync

  assert_status 58
  assert_contains "$run_stderr" "gwt: not inside a git repository"
  assert_stdout_empty
}

test_env_remove_and_rm_outside_repo_returns_58() {
  cd "$SANDBOX/work"

  invoke_gwt env remove .env

  assert_status 58
  assert_contains "$run_stderr" "gwt: not inside a git repository"

  invoke_gwt env rm

  assert_status 58
  assert_contains "$run_stderr" "gwt: not inside a git repository"
}

# ---------------------------------------------------------------------------
# env add
# ---------------------------------------------------------------------------

test_env_add_tracks_file_in_main_repo_gwt_envs() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt env add .env

  assert_status 0
  assert_eq "$repo/.gwt/envs" "$run_stdout"
  assert_file_contains "$repo/.gwt/envs" ".env"
}

test_env_add_from_worktree_creates_worktree_local_config() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  cd "$wt"

  invoke_gwt env add .env

  assert_status 0
  assert_eq "$wt/.gwt/envs" "$run_stdout"
  assert_file_contains "$wt/.gwt/envs" ".env"
  assert_file_absent "$repo/.gwt/envs"
}

test_env_add_u_tracks_file_in_user_config() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt env add -u .env

  assert_status 0
  assert_eq "$XDG_CONFIG_HOME/gwt/envs/demo" "$run_stdout"
  assert_file_contains "$XDG_CONFIG_HOME/gwt/envs/demo" ".env"
}

test_env_add_appends_to_existing_list() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt env add .env
  assert_status 0
  invoke_gwt env add .env.local
  assert_status 0

  assert_file_contains "$repo/.gwt/envs" ".env"
  assert_file_contains "$repo/.gwt/envs" ".env.local"
}

test_env_add_accepts_files_that_do_not_exist_yet() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt env add secrets.env

  assert_status 0
  assert_file_contains "$repo/.gwt/envs" "secrets.env"
}

# ---------------------------------------------------------------------------
# env list / env ls
# ---------------------------------------------------------------------------

test_env_list_prints_config_path_and_tracked_files() {
  local repo=$(make_repo demo)
  echo "SECRET=1" > "$repo/.env"
  cd "$repo"

  invoke_gwt env add .env
  invoke_gwt env list

  assert_status 0
  assert_contains "$run_stdout" "$repo/.gwt/envs"
  assert_contains "$run_stdout" "environment files:"
  assert_contains "$run_stderr" ".env"
}

test_env_ls_alias_behaves_like_list() {
  local repo=$(make_repo demo)
  echo "SECRET=1" > "$repo/.env"
  cd "$repo"

  invoke_gwt env add .env
  invoke_gwt env ls

  assert_status 0
  assert_contains "$run_output" "$repo/.gwt/envs"
  assert_contains "$run_output" "environment files:"
  assert_contains "$run_output" ".env"
}

test_env_list_without_config_reports_none() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt env list

  assert_status 0
  assert_contains "$run_stdout" "gwt: no env configured for demo"
}

test_env_list_ignores_files_missing_from_main_repo() {
  local repo=$(make_repo demo)
  echo "SECRET=1" > "$repo/.env"
  cd "$repo"

  invoke_gwt env add .env
  invoke_gwt env add .env.missing
  invoke_gwt env list

  assert_status 0
  assert_contains "$run_output" ".env"
  assert_not_contains "$run_output" ".env.missing"
}

test_env_list_deduplicates_repeated_entries() {
  local repo=$(make_repo demo)
  echo "SECRET=1" > "$repo/.env"
  cd "$repo"

  invoke_gwt env add .env
  invoke_gwt env add .env
  invoke_gwt env list

  assert_status 0
  local entries=$(gwt env list 2>&1 >/dev/null)
  assert_eq 1 $(print -r -- "$entries" | grep -c '^\.env$')
}

test_env_list_prefers_worktree_local_config_over_main() {
  local repo=$(make_repo demo)
  echo "SECRET=1" > "$repo/.env"
  echo "LOCAL=1" > "$repo/.env.local"
  git -C "$repo" add .env.local >/dev/null
  git -C "$repo" commit -qm "add .env.local"
  local wt=$(add_worktree "$repo" feature)
  cd "$wt"

  invoke_gwt env add .env.local
  invoke_gwt env list

  assert_status 0
  assert_contains "$run_stdout" "$wt/.gwt/envs"
  assert_not_contains "$run_stdout" "$repo/.gwt/envs"
  assert_contains "$run_output" ".env.local"
}

test_env_list_user_config_overrides_local_with_warning() {
  local repo=$(make_repo demo)
  echo "SECRET=1" > "$repo/.env"
  echo "LOCAL=1" > "$repo/.env.local"
  git -C "$repo" add .env.local >/dev/null
  git -C "$repo" commit -qm "add .env.local"
  cd "$repo"

  invoke_gwt env add .env
  invoke_gwt env add -u .env.local
  invoke_gwt env list

  assert_status 0
  assert_contains "$run_output" "gwt: WARN configured env is overriden by user env!"
  assert_contains "$run_stdout" "$XDG_CONFIG_HOME/gwt/envs/demo"
  assert_contains "$run_output" ".env.local"
}

# ---------------------------------------------------------------------------
# env sync
# ---------------------------------------------------------------------------

test_env_sync_copies_tracked_files_into_worktree() {
  local repo=$(make_repo demo)
  echo "SECRET=1" > "$repo/.env"
  local wt=$(add_worktree "$repo" feature)
  cd "$repo"
  invoke_gwt env add .env

  cd "$wt"
  invoke_gwt env sync

  assert_status 0
  assert_contains "$run_stdout" "Copying files:"
  assert_file_contains "$wt/.env" "SECRET=1"
}

# Files missing from the source directory are silently skipped by the filter
# loop before the copy step, so the "WARN unable to copy env" branch is never
# reached. This test pins that ACTUAL behaviour (silent skip, no warning).
test_env_sync_silently_skips_missing_source_files() {
  local repo=$(make_repo demo)
  echo "SECRET=1" > "$repo/.env"
  local wt=$(add_worktree "$repo" feature)
  cd "$repo"
  invoke_gwt env add .env
  invoke_gwt env add .env.missing

  cd "$wt"
  invoke_gwt env sync

  assert_status 0
  assert_file_contains "$wt/.env" "SECRET=1"
  assert_not_contains "$run_output" "unable to copy env .env.missing"
  assert_file_absent "$wt/.env.missing"
}

test_env_sync_without_config_reports_none() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  cd "$wt"

  invoke_gwt env sync

  assert_status 0
  assert_contains "$run_output" "gwt: no env configured for demo"
}

test_env_sync_from_worktree_uses_worktree_local_config() {
  local repo=$(make_repo demo)
  echo "SECRET=1" > "$repo/.env"
  echo "LOCAL=1" > "$repo/.env.local"
  git -C "$repo" add .env.local >/dev/null
  git -C "$repo" commit -qm "add .env.local"
  local wt=$(add_worktree "$repo" feature)
  cd "$wt"

  invoke_gwt env add .env.local
  invoke_gwt env sync

  assert_status 0
  assert_file_contains "$wt/.env.local" "LOCAL=1"
  assert_file_absent "$wt/.env"
}

# `env sync <path>` is documented as syncing from another location, but the
# implementation's `$env_src=$1` is a failed command, not an assignment, so
# the source directory stays at the main repo. This test pins that ACTUAL
# behaviour; see the report note about the README discrepancy.
test_env_sync_with_explicit_source_path_uses_given_dir() {
  local repo=$(make_repo demo)
  echo "SECRET=1" > "$repo/.env"
  local wt=$(add_worktree "$repo" feature)
  local other="$SANDBOX/other-src"
  mkdir -p "$other"
  echo "OTHER=2" > "$other/.env"
  cd "$wt"
  invoke_gwt env add .env

  invoke_gwt env sync "$other"

  assert_status 0
  assert_contains "$run_stdout" "gwt: using source dir $other"
  assert_file_contains "$wt/.env" "OTHER=2"
  assert_not_contains "$(<"$wt/.env")" "SECRET=1"
}

# There is no fall-through to sync: an unrecognised subcommand is an error.
test_env_unknown_subcommand_returns_61() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  cd "$wt"

  invoke_gwt env bogus

  assert_status 61
  assert_contains "$run_stderr" "unknown command: env bogus"
}

test_env_missing_subcommand_returns_61() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  cd "$wt"

  invoke_gwt env

  assert_status 61
  assert_contains "$run_stderr" "missing subcommand"
}

# ---------------------------------------------------------------------------
# env remove / env rm
# ---------------------------------------------------------------------------

test_env_remove_single_file_keeps_others() {
  local repo=$(make_repo demo)
  echo "SECRET=1" > "$repo/.env"
  echo "LOCAL=1" > "$repo/.env.local"
  cd "$repo"

  invoke_gwt env add .env
  invoke_gwt env add .env.local
  invoke_gwt env remove .env.local

  assert_status 0
  assert_eq "$(<"$repo/.gwt/envs")" ".env"
}

test_env_remove_all_deletes_config_file() {
  local repo=$(make_repo demo)
  echo "SECRET=1" > "$repo/.env"
  cd "$repo"

  invoke_gwt env add .env
  invoke_gwt env remove

  assert_status 0
  assert_contains "$run_stdout" "gwt: Deleting $repo/.gwt/envs"
  assert_file_absent "$repo/.gwt/envs"
}

test_env_remove_without_any_config_reports_nothing_to_remove() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt env remove

  assert_status 0
  assert_contains "$run_stdout" "gwt: nothing to remove"
}

test_env_remove_from_worktree_without_local_config_leaves_main_untouched() {
  local repo=$(make_repo demo)
  echo "SECRET=1" > "$repo/.env"
  local wt=$(add_worktree "$repo" feature)
  cd "$repo"
  invoke_gwt env add .env

  cd "$wt"
  invoke_gwt env remove .env

  assert_status 0
  assert_file_contains "$repo/.gwt/envs" ".env"
}

test_env_remove_u_single_file_updates_user_config() {
  local repo=$(make_repo demo)
  echo "SECRET=1" > "$repo/.env"
  echo "LOCAL=1" > "$repo/.env.local"
  cd "$repo"

  invoke_gwt env add -u .env
  invoke_gwt env add -u .env.local
  invoke_gwt env remove -u .env.local

  assert_status 0
  assert_file_contains "$XDG_CONFIG_HOME/gwt/envs/demo" ".env"
  assert_not_contains "$(<"$XDG_CONFIG_HOME/gwt/envs/demo")" ".env.local"
}

test_env_remove_u_all_deletes_user_config() {
  local repo=$(make_repo demo)
  echo "SECRET=1" > "$repo/.env"
  cd "$repo"

  invoke_gwt env add -u .env
  invoke_gwt env remove -u

  assert_status 0
  assert_contains "$run_stdout" "gwt: Deleting $XDG_CONFIG_HOME/gwt/envs/demo"
  assert_file_absent "$XDG_CONFIG_HOME/gwt/envs/demo"
}

test_env_rm_alias_removes_single_file() {
  local repo=$(make_repo demo)
  echo "SECRET=1" > "$repo/.env"
  echo "LOCAL=1" > "$repo/.env.local"
  cd "$repo"

  invoke_gwt env add .env
  invoke_gwt env add .env.local
  invoke_gwt env rm .env

  assert_status 0
  assert_eq "$(<"$repo/.gwt/envs")" ".env.local"
}
