#!/usr/bin/env zsh
# Tests for utils/jq-inplace.sh (_gwt_jq_inplace).
#
# Contract:
#  - rewrites the target file in place with the jq program applied
#  - preserves the target's permissions (mktemp creates mode-0600 files, so
#    without an explicit chmod the replacement would silently drop the mode)
#  - leaves no temp file behind when jq fails
#  - returns non-zero when jq fails, and leaves the target untouched

source "$REPO_ROOT/utils/jq-inplace.sh"

test_rewrites_file_in_place() {
  local file="$SANDBOX/devcontainer.json"
  print -r -- '{"name":"demo","mounts":[]}' > "$file"

  _gwt_jq_inplace "$file" '.name = "renamed"'
  assert_status 0

  local content=$(<"$file")
  assert_contains "$content" '"renamed"'
}

test_preserves_file_permissions() {
  local file="$SANDBOX/devcontainer.json"
  print -r -- '{"name":"demo"}' > "$file"
  chmod 664 "$file"

  _gwt_jq_inplace "$file" '.name = "renamed"'
  assert_status 0

  # BSD stat (%Lp) and GNU stat (%a) both print the octal mode; check GNU
  # first since BSD stat rejects -c but GNU stat accepts -f as --file-system
  local mode=$(stat -c '%a' "$file" 2>/dev/null || stat -f '%Lp' "$file")
  assert_eq "664" "$mode"
}

test_jq_failure_leaves_no_temp_file() {
  local file="$SANDBOX/devcontainer.json"
  print -r -- '{"name":"demo"}' > "$file"

  # an invalid jq program fails after the temp file exists
  local rc=0
  _gwt_jq_inplace "$file" 'not a valid jq program' || rc=$?
  (( rc != 0 )) || _assert_fail "expected _gwt_jq_inplace to fail"

  # the target must be untouched and no *.jqtmp.* left in the directory
  local content=$(<"$file")
  assert_contains "$content" '"demo"'
  local -a leftovers=("$SANDBOX"/*.jqtmp.*(N))
  assert_eq 0 "${#leftovers}"
}

test_missing_target_fails_without_temp_file() {
  local file="$SANDBOX/absent.json"

  local rc=0
  _gwt_jq_inplace "$file" '.name = "renamed"' || rc=$?
  (( rc != 0 )) || _assert_fail "expected _gwt_jq_inplace to fail"
  assert_file_absent "$file"
  local -a leftovers=("$SANDBOX"/*.jqtmp.*(N))
  assert_eq 0 "${#leftovers}"
}
