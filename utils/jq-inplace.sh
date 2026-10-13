#!/bin/zsh
_gwt_jq_inplace() {
    local file=$1
    shift
    local tmp=$(mktemp)
    jq "$@" "$file" > "$tmp" && mv "$tmp" "$file"
}
