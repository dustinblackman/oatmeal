#!/usr/bin/env bash

set -e

PROGDIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

cargo about generate -c "$PROGDIR/about.toml" "$PROGDIR/templates/html.hbs" >"$PROGDIR/../../debug.html"
open "$PROGDIR/../../debug.html"
