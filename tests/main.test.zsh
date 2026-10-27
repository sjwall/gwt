#!/usr/bin/env zsh
# Tests for `gwt main` / `gwt m` / `gwt M` (commands/main.sh).
#
# Exit code contract:
#   5  - not inside a git repository and no repository specified or found
#   6  - invalid argument count (more than 1 argument)
#   7  - multiple exact repository matches
#   8  - multiple repository name matches
#   9  - multiple repository path matches
#   10 - no matching repository for query
#   11 - missing required argument for `--ide` (M only)

test_main_from_main_repo_prints_path_and_stays() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt main

  assert_status 0
  assert_eq "$repo" "$run_stdout"
  assert_pwd "$repo"
}

test_main_from_worktree_returns_to_main_repo() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  cd "$wt"

  invoke_gwt main

  assert_status 0
  assert_eq "$repo" "$run_stdout"
  assert_pwd "$repo"
}

test_main_outside_repo_with_no_tracked_repos_returns_5() {
  # setup_sandbox starts in $SANDBOX/work, which is not a git repository.
  invoke_gwt main

  assert_status 5
  assert_contains "$run_stderr" "no matching repository found"
  assert_stdout_empty
}

test_main_outside_repo_auto_switches_single_tracked_repo() {
  local repo=$(make_repo demo)
  track_repo "$repo"

  invoke_gwt main

  assert_status 0
  assert_eq "$repo" "$run_stdout"
  assert_pwd "$repo"
}

test_main_ignores_missing_tracked_paths() {
  local repo=$(make_repo demo)
  track_repo "$repo"
  track_repo "$SANDBOX/repos/does-not-exist"

  invoke_gwt main

  assert_status 0
  assert_pwd "$repo"
}

test_main_deduplicates_tracked_list() {
  local repo=$(make_repo demo)
  track_repo "$repo"
  track_repo "$repo"

  invoke_gwt main

  assert_status 0
  assert_pwd "$repo"
}

test_main_with_query_matches_exact_repo_name() {
  local demo=$(make_repo demo)
  local other=$(make_repo other)
  track_repo "$demo"
  cd "$other"

  invoke_gwt main demo

  assert_status 0
  assert_eq "$demo" "$run_stdout"
  assert_pwd "$demo"
}

test_main_query_is_case_insensitive() {
  local demo=$(make_repo demo)
  local other=$(make_repo other)
  track_repo "$demo"
  cd "$other"

  invoke_gwt main DEMO

  assert_status 0
  assert_pwd "$demo"
}

test_main_query_matches_unique_name_substring() {
  local demo=$(make_repo demo)
  local other=$(make_repo other)
  track_repo "$demo"
  cd "$other"

  invoke_gwt main dem

  assert_status 0
  assert_pwd "$demo"
}

test_main_prefers_exact_match_over_substring() {
  local demo=$(make_repo demo)
  make_repo demo-extra >/dev/null
  track_repo "$demo"
  track_repo "$SANDBOX/repos/demo-extra"

  invoke_gwt main demo

  assert_status 0
  assert_pwd "$demo"
}

test_main_finds_current_repo_main_even_when_untracked() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  cd "$wt"

  invoke_gwt main demo

  assert_status 0
  assert_pwd "$repo"
}

test_main_matches_by_path_substring() {
  local repo=$(make_repo demo)
  track_repo "$repo"

  invoke_gwt main "repos/demo"

  assert_status 0
  assert_pwd "$repo"
}

test_main_multiple_exact_matches_returns_7() {
  local one=$(make_repo a/demo)
  local two=$(make_repo b/demo)
  track_repo "$one"
  track_repo "$two"

  invoke_gwt main demo

  assert_status 7
  assert_contains "$run_stderr" "multiple repositories match 'demo'"
  assert_contains "$run_stderr" "$one"
  assert_contains "$run_stderr" "$two"
  assert_stdout_empty
}

test_main_multiple_name_matches_returns_8() {
  local one=$(make_repo alpha-demo)
  local two=$(make_repo beta-demo)
  track_repo "$one"
  track_repo "$two"

  invoke_gwt main demo

  assert_status 8
  assert_contains "$run_stderr" "multiple repositories match 'demo'"
  assert_contains "$run_stderr" "$one"
  assert_contains "$run_stderr" "$two"
  assert_stdout_empty
}

test_main_multiple_path_matches_returns_9() {
  local one=$(make_repo alpha)
  local two=$(make_repo beta)
  track_repo "$one"
  track_repo "$two"

  invoke_gwt main "repos"

  assert_status 9
  assert_contains "$run_stderr" "multiple repositories match 'repos'"
  assert_contains "$run_stderr" "$one"
  assert_contains "$run_stderr" "$two"
  assert_stdout_empty
}

test_main_no_match_returns_10() {
  local repo=$(make_repo demo)
  track_repo "$repo"

  invoke_gwt main nope

  assert_status 10
  assert_contains "$run_stderr" "no matching repository found for 'nope'"
  assert_stdout_empty
}

test_main_too_many_arguments_returns_6() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt main demo extra

  assert_status 6
  assert_contains "$run_stderr" "unknown command: 'main demo extra'"
}

test_M_switches_and_launches_ide() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  local log=$(fake_ide)
  cd "$wt"

  invoke_gwt M

  assert_status 0
  assert_pwd "$repo"
  assert_file_contains "$log" "$repo"
}

test_M_with_query_switches_and_launches_ide() {
  local demo=$(make_repo demo)
  local other=$(make_repo other)
  track_repo "$demo"
  local log=$(fake_ide)
  cd "$other"

  invoke_gwt M demo

  assert_status 0
  assert_pwd "$demo"
  assert_file_contains "$log" "$demo"
}

test_M_ide_none_skips_launch() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  local log=$(fake_ide)
  cd "$wt"

  invoke_gwt M --ide none

  assert_status 0
  assert_pwd "$repo"
  assert_file_absent "$log"
}

test_M_ide_override_launches_named_ide() {
  local repo=$(make_repo demo)
  local log=$(fake_cmd other-ide)
  cd "$repo"

  invoke_gwt M --ide other-ide

  assert_status 0
  assert_file_contains "$log" "$repo"
  assert_file_absent "$SANDBOX/bin/fake-ide.log"
}

test_M_missing_ide_argument_returns_11() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt M --ide

  assert_status 11
  assert_contains "$run_stderr" "--ide requires an argument"
}

test_M_too_many_arguments_returns_6() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt M demo extra

  assert_status 6
  assert_contains "$run_stderr" "unknown command: 'M demo extra'"
}

test_M_propagates_main_error_status() {
  local repo=$(make_repo demo)
  track_repo "$repo"

  invoke_gwt M nope

  assert_status 10
  assert_contains "$run_stderr" "no matching repository found for 'nope'"
}
