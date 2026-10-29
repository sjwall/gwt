#!/usr/bin/env zsh
# Tests for `gwt config` / `gwt ide` (commands/config.sh).
#
# Exit code contract:
#   29 - invalid argument count for `config get` (expected exactly 1 key)
#   30 - specified configuration key not found in `config get`
#   31 - invalid argument count for `config set` (expected key and value)
#   32 - invalid argument count for `config unset` (expected exactly 1 key)
#   33 - specified configuration key not found in `config <key>`

_config_file() {
  print -r -- "$XDG_CONFIG_HOME/gwt/config"
}

test_config_shows_default_ide_when_empty() {
  invoke_gwt config

  assert_status 0
  assert_contains "$run_output" "ide=nvim (default)"
}

test_config_shows_file_contents() {
  invoke_gwt config set ide code >/dev/null
  invoke_gwt config set agent cursor >/dev/null

  invoke_gwt config

  assert_status 0
  assert_contains "$run_output" "ide=code"
  assert_contains "$run_output" "agent=cursor"
}

test_config_get_returns_set_value() {
  invoke_gwt config set theme dark

  invoke_gwt config get theme

  assert_status 0
  assert_eq "dark" "$run_stdout"
}

test_config_get_missing_key_returns_30() {
  invoke_gwt config get nope

  assert_status 30
  assert_contains "$run_stderr" "config key 'nope' not found"
  assert_stdout_empty
}

test_config_get_wrong_arg_count_returns_29() {
  invoke_gwt config get

  assert_status 29
  assert_contains "$run_stderr" "usage: gwt config get <key>"

  invoke_gwt config get a b

  assert_status 29
  assert_stdout_empty
}

test_config_get_ide_prefers_config_over_environment() {
  # GWT_IDE=none is exported in the sandbox; an explicit config wins.
  invoke_gwt config get ide

  assert_status 0
  assert_eq "none" "$run_stdout"

  invoke_gwt config set ide code >/dev/null

  invoke_gwt config get ide

  assert_status 0
  assert_eq "code" "$run_stdout"
}

test_config_get_agent_prefers_config_over_environment() {
  # GWT_AGENT=none is exported in the sandbox; an explicit config wins.
  invoke_gwt config get agent

  assert_status 0
  assert_eq "none" "$run_stdout"

  invoke_gwt config set agent cursor >/dev/null

  invoke_gwt config get agent

  assert_status 0
  assert_eq "cursor" "$run_stdout"
}

test_config_set_writes_config_file() {
  invoke_gwt config set ide code

  assert_status 0
  assert_contains "$run_output" "gwt: set ide to code"
  assert_file_contains "$(_config_file)" "ide=code"
}

test_config_set_joins_multiple_value_arguments() {
  invoke_gwt config set editor code --wait

  assert_status 0

  invoke_gwt config get editor

  assert_status 0
  assert_eq "code --wait" "$run_stdout"
}

test_config_set_overwrites_existing_key() {
  invoke_gwt config set ide code >/dev/null
  invoke_gwt config set ide vim >/dev/null

  invoke_gwt config get ide

  assert_status 0
  assert_eq "vim" "$run_stdout"
  assert_eq "1" "$(grep -Fxc "ide=vim" "$(_config_file)")"
}

test_config_set_wrong_arg_count_returns_31() {
  invoke_gwt config set

  assert_status 31
  assert_contains "$run_stderr" "usage: gwt config set <key> <value>"

  invoke_gwt config set onlykey

  assert_status 31
  assert_stdout_empty
}

test_config_unset_removes_key() {
  invoke_gwt config set theme dark >/dev/null

  invoke_gwt config unset theme

  assert_status 0
  assert_contains "$run_output" "gwt: unset theme"

  invoke_gwt config get theme

  assert_status 30
  assert_not_contains "$(_config_file)" "theme=dark"
}

test_config_unset_missing_key_succeeds() {
  invoke_gwt config unset nope

  assert_status 0
  assert_contains "$run_output" "gwt: unset nope"
}

test_config_unset_wrong_arg_count_returns_32() {
  invoke_gwt config unset

  assert_status 32
  assert_contains "$run_stderr" "usage: gwt config unset <key>"

  invoke_gwt config unset a b

  assert_status 32
  assert_stdout_empty
}

test_config_unset_rm_alias_works() {
  invoke_gwt config set theme dark >/dev/null

  invoke_gwt config rm theme

  assert_status 0

  invoke_gwt config get theme

  assert_status 30
}

test_config_shorthand_gets_key() {
  invoke_gwt config set theme dark >/dev/null

  invoke_gwt config theme

  assert_status 0
  assert_eq "dark" "$run_stdout"
}

test_config_shorthand_missing_key_returns_33() {
  invoke_gwt config nope

  assert_status 33
  assert_contains "$run_stderr" "config key 'nope' not found"
  assert_stdout_empty
}

test_config_shorthand_sets_key() {
  invoke_gwt config theme dark

  assert_status 0
  assert_contains "$run_output" "gwt: set theme to dark"
  assert_file_contains "$(_config_file)" "theme=dark"
}

test_ide_shorthand_gets_and_sets_ide() {
  invoke_gwt config set ide code >/dev/null

  invoke_gwt ide

  assert_status 0
  assert_eq "code" "$run_stdout"

  invoke_gwt ide vim

  assert_status 0

  invoke_gwt ide

  assert_status 0
  assert_eq "vim" "$run_stdout"
  assert_file_contains "$(_config_file)" "ide=vim"
}
