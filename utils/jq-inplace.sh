#!/bin/zsh
# _gwt_jq_inplace <file> <jq-program> [jq-args...]
#
# Rewrites <file> in place with jq. The temp file is created next to the
# target so the final mv is an atomic same-filesystem rename, it inherits the
# target's permissions (mktemp creates mode-0600 files), and it is removed on
# every failure path, including signals.
_gwt_jq_inplace() {
    local file=$1
    shift

    local tmp
    tmp=$(mktemp "${file}.jqtmp.XXXXXX") || return $?

    # Clean up the temp file if we are interrupted while holding it.
    trap "rm -f -- ${(q)tmp}" INT TERM HUP

    local rc=0
    if jq "$@" "$file" > "$tmp"; then
        # Preserve the original permissions; mktemp creates mode-0600 files.
        local mode
        if mode=$(stat -f '%Lp' "$file" 2>/dev/null || stat -c '%a' "$file" 2>/dev/null) \
            && chmod "$mode" "$tmp" \
            && mv -f -- "$tmp" "$file"; then
            tmp=""
        else
            rc=$?
        fi
    else
        rc=$?
    fi

    trap - INT TERM HUP
    [[ -z "$tmp" ]] || rm -f -- "$tmp"
    return $rc
}
