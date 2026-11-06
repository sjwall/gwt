#!/usr/bin/env zsh
# Tests for lock acquisition failure exit codes.
#
# gwt takes the repos.lock at startup (gwt.sh) and again inside some
# subcommands. A held lock fails after the 30s timeout with a command
# specific code:
#
# Exit code contract:
#  74 - gwt startup lock acquisition failed
#
# The subcommand specific lock codes (67, 69-73, 75) are only reachable when
# the lock appears between the startup release and the subcommand's own
# acquire, which cannot be triggered deterministically from the CLI; they
# share the same acquisition code path as the startup lock tested here.

test_startup_lock_timeout_returns_74() {
  local repo=$(make_repo demo)
  cd "$repo"

  # a lock held by another (or crashed) process blocks the startup lock
  mkdir -p "$XDG_CONFIG_HOME/gwt"
  echo 99999 > "$XDG_CONFIG_HOME/gwt/repos.lock"

  # advance time on every `date` call so the 30s lock timeout elapses
  # after a single 0.1s wait instead of really blocking for 30 seconds
  local tick=0
  date() {
    tick=$((tick + 100))
    builtin echo $(( $(/bin/date +%s) + tick ))
  }

  invoke_gwt ls

  assert_status 74
  assert_contains "$run_stderr" "gwt: timeout waiting for lock"
  # the foreign lock file must be left alone (release only removes own locks)
  assert_file_contains "$XDG_CONFIG_HOME/gwt/repos.lock" "99999"
}
