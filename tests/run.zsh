#!/usr/bin/env zsh
# Dependency-free test runner for gwt.
#
# Usage:
#   tests/run.zsh                    run every tests/*.test.zsh
#   tests/run.zsh tests/cd.test.zsh  run specific files
#
# Set GWT_TEST_KEEP=1 to keep the per-test sandbox directories for debugging.
#
# Test files define functions named test_<description>. Each test runs in its
# own subshell with an isolated HOME/XDG/GWT_DIR sandbox; see test-helpers.zsh.

emulate -L zsh

typeset -r TESTS_DIR=${0:A:h}
typeset -r REPO_ROOT=${TESTS_DIR:h}
export REPO_ROOT

local -a files
if (( $# > 0 )); then
  files=("$@")
else
  files=(${TESTS_DIR}/*.test.zsh(N))
fi

local -i total=0 failed=0 runner_errors=0
local file output line
for file in $files; do
  if [[ ! -f "$file" ]]; then
    print -ru2 -- "# no such test file: $file"
    (( runner_errors++ ))
    continue
  fi

  print -r -- "# ${file:t}"

  output=$(zsh -f "${TESTS_DIR}/run-file.zsh" "$file" 2>&1)
  local file_status=$?

  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    print -r -- "$line"
    case "$line" in
      'ok - '*) (( total++ )) ;;
      'not ok - '*)
        (( total++ ))
        (( failed++ ))
        ;;
    esac
  done <<< "$output"

  if (( file_status != 0 )) && [[ "$output" != *'not ok - '* ]]; then
    print -r -- "not ok - ${file:t} (runner error)"
    (( total++ ))
    (( failed++ ))
    (( runner_errors++ ))
  fi
done

print -r -- "1..$total"
if (( failed == 0 && runner_errors == 0 )); then
  print -r -- "# ${total} tests, all passed"
  exit 0
fi

print -r -- "# ${total} tests, $(( total - failed )) passed, ${failed} failed"
exit 1
