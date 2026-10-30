#!/usr/bin/env zsh
# Tests for `gwt migrate` (commands/migrate.sh).
#
# Exit code contract:
#   52 - not inside a git repository
#   53 - unknown argument or option
#   54 - failed to determine target worktree directory location
#   55 - `git worktree move` command failed

test_migrate_outside_a_git_repository_returns_52() {
  # $PWD starts in $SANDBOX/work, which is not a git repository.
  invoke_gwt migrate

  assert_status 52
  assert_contains "$run_stderr" "not inside a git repository"
  assert_stdout_empty
}

test_migrate_unknown_argument_returns_53() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt migrate extra-arg

  assert_status 53
  assert_contains "$run_stderr" "unknown command: 'migrate extra-arg'"
}

test_migrate_unknown_option_returns_53() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt migrate --bogus

  assert_status 53
  assert_contains "$run_stderr" "unknown command: 'migrate --bogus'"
}

