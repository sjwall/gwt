#!/usr/bin/env zsh
# Runs every test_* function in a single test file. Invoked by run.zsh.
# Each test executes in a subshell so `cd`, globals, and traps cannot leak.

emulate -L zsh

typeset -r TESTS_DIR=${0:A:h}
export REPO_ROOT=${REPO_ROOT:-${TESTS_DIR:h}}

source "${TESTS_DIR}/test-helpers.zsh"

local file=$1
if [[ ! -f "$file" ]]; then
  print -ru2 -- "no such test file: $file"
  exit 1
fi

source "$file"

local -a tests=()
local f
for f in ${(k)functions}; do
  [[ "$f" == test_* ]] && tests+=("$f")
done
tests=(${(o)tests})
if (( ${#tests} == 0 )); then
  print -ru2 -- "no test_* functions found in ${file:t}"
  exit 1
fi

local -i failed=0
local t name diag st
for t in $tests; do
  name=${t#test_}
  name=${name//_/ }

  diag=$(mktemp "${TMPDIR:-/tmp}/gwt-test-diag.XXXXXX") || exit 1

  (
    setup_sandbox
    # Set the trap at subshell scope, not inside a function: a zsh EXIT trap
    # defined inside a function fires when that function returns.
    trap '(( ${GWT_TEST_KEEP:-0} )) || rm -rf -- "$SANDBOX"' EXIT
    "$t"
  ) >"$diag" 2>&1
  st=$?

  if (( st == 0 )); then
    print -r -- "ok - $name"
  else
    print -r -- "not ok - $name"
    sed 's/^/# /' "$diag"
    (( failed++ ))
  fi

  rm -f "$diag"
done

(( failed == 0 ))
