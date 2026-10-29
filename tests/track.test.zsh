#!/usr/bin/env zsh
# Tests for `gwt track` / `gwt t` (commands/track.sh, track-add.sh,
# track-explore.sh, track-prune.sh).
#
# Exit code contract:
#   36 - not inside a git repository and no repository specified
#   37 - invalid argument count (more than 1 argument, or args with --explore)
#   38 - specified path is not a git repository
#   59 - unknown argument
#   60 - --explore run from inside a git repository

_repos_file() {
  print -r -- "$XDG_CONFIG_HOME/gwt/repos"
}

test_track_tracks_current_repository() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt track

  assert_status 0
  assert_contains "$run_output" "$repo"
  assert_file_contains "$(_repos_file)" "$repo"
}

test_track_shorthand_t_works() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt t

  assert_status 0
  assert_contains "$run_output" "$repo"
}

test_track_outside_repo_returns_36() {
  invoke_gwt track

  assert_status 36
  assert_contains "$run_stderr" "not inside a git repository"
  assert_stdout_empty
}

test_track_explicit_path() {
  local other=$(make_repo other)

  invoke_gwt track "$other"

  assert_status 0
  assert_contains "$run_output" "$other"
  assert_file_contains "$(_repos_file)" "$other"
}

test_track_worktree_path_records_main_repo() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)

  invoke_gwt track "$wt"

  assert_status 0
  assert_contains "$run_output" "$repo"
  assert_file_contains "$(_repos_file)" "$repo"
  assert_not_contains "$(_repos_file)" "$wt"
}

test_track_nonexistent_path_returns_38() {
  invoke_gwt track "$SANDBOX/repos/does-not-exist"

  assert_status 38
  assert_contains "$run_stderr" "not a git repository"
}

test_track_non_git_directory_returns_38() {
  mkdir -p "$SANDBOX/work/plain"

  invoke_gwt track "$SANDBOX/work/plain"

  assert_status 38
  assert_contains "$run_stderr" "not a git repository"
}

test_track_too_many_arguments_returns_37() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt track "$repo" extra

  assert_status 37
  assert_contains "$run_stderr" "unknown command: 'track $repo extra'"
}

test_track_is_idempotent() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt track
  invoke_gwt track

  assert_status 0
  assert_eq "1" "$(grep -Fxc "$repo" "$(_repos_file)")"
}

test_track_explore_finds_nested_repos() {
  local one=$(make_repo demo)
  local two=$(make_repo deep/nested/other)
  cd "$SANDBOX"

  invoke_gwt track --explore

  assert_status 0
  assert_contains "$run_output" "$one"
  assert_contains "$run_output" "$two"
  assert_file_contains "$(_repos_file)" "$one"
  assert_file_contains "$(_repos_file)" "$two"
}

test_track_explore_skips_inside_found_repos() {
  local outer=$(make_repo outer)
  git -C "$outer" init -q "$outer/inner"
  cd "$SANDBOX"

  invoke_gwt track --explore

  assert_status 0
  assert_contains "$run_output" "$outer"
  assert_not_contains "$(_repos_file)" "$outer/inner"
}

test_track_explore_inside_repo_returns_60() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt track --explore

  assert_status 60
  assert_contains "$run_stderr" "already a git repository"
}

test_track_explore_with_arguments_returns_37() {
  invoke_gwt track --explore foo

  assert_status 37
  assert_contains "$run_stderr" "does not accept arguments"
}

test_track_explore_no_repos_found_is_not_an_error() {
  mkdir -p "$SANDBOX/work/empty"
  cd "$SANDBOX/work/empty"

  invoke_gwt track --explore

  assert_status 0
  assert_contains "$run_stderr" "no git repositories found"
}

test_track_prune_removes_missing_paths() {
  local repo=$(make_repo demo)
  track_repo "$repo"
  track_repo "$SANDBOX/repos/does-not-exist"

  invoke_gwt track --prune

  assert_status 0
  assert_file_contains "$(_repos_file)" "$repo"
  assert_not_contains "$(_repos_file)" "does-not-exist"
}

test_track_prune_keeps_existing_repos() {
  local one=$(make_repo one)
  local two=$(make_repo two)
  track_repo "$one"
  track_repo "$two"

  invoke_gwt track --prune

  assert_status 0
  assert_file_contains "$(_repos_file)" "$one"
  assert_file_contains "$(_repos_file)" "$two"
}

test_track_prune_dry_run_keeps_file() {
  local repo=$(make_repo demo)
  track_repo "$repo"
  track_repo "$SANDBOX/repos/does-not-exist"

  invoke_gwt track --prune --dry-run

  assert_status 0
  assert_contains "$run_output" "Repos to remove:"
  assert_contains "$run_output" "does-not-exist"
  assert_file_contains "$(_repos_file)" "$repo"
  assert_file_contains "$(_repos_file)" "does-not-exist"
}

test_track_unknown_flag_returns_59() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt track --bogus

  assert_status 59
  assert_contains "$run_stderr" "unknown command: track --bogus"
}
