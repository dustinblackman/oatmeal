#!/usr/bin/env bash

set -e

PROGDIR=$(cd "$(dirname "$0")" && pwd)
cd "$PROGDIR"

VERSION="$1"
DIST="$2"
TMPDIR="$(mktemp -d)"

cd "$TMPDIR"
git clone --depth 1 git@github.com:dustinblackman/chocolatey-packages.git .
./add-package.sh oatmeal "$VERSION" "$DIST" "$(realpath "$PROGDIR"/../.goreleaser.yml)"
cd ~
rm -rf "$TMPDIR"
