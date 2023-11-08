#!/usr/bin/env bash

set -e

PROGDIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

crate="$(echo "$1" | awk -F '--' '{print $1}')"
license="$2"
url=$(echo "$3" | sd 'github.com' 'raw.githubusercontent.com' | sd 'blob/' '')

checksum=$(curl -s -L "$url" | shasum -a 256 | awk '{print $1}')
filename=$(basename "$url")

tomldata="

[${crate}.clarify]
license = \"${license}\"

[[${crate}.clarify.git]]
path = \"${filename}\"
checksum = \"${checksum}\""

echo "$tomldata" >>"$PROGDIR/about.toml"

cargo bin dprint fmt
