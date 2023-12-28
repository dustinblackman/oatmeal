#!/usr/bin/env bash

set -e

VERSION="$1"
DIST="$2"
TMPDIR="$(mktemp -d)"

cd "$TMPDIR"
git clone --depth 1 git@github.com:dustinblackman/nur-packages.git .
./scripts/goreleaser-v2.sh oatmeal "$VERSION" "$DIST"
cd ~
rm -rf "$TMPDIR"
