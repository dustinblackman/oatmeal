#!/usr/bin/env bash

set -e

VERSION="$1"
DIST="$2"
TMPDIR="$(mktemp -d)"

cd "$TMPDIR"
git clone --depth 1 git@github.com:dustinblackman/yum.git .
./add-rpms.sh oatmeal "$VERSION" "$DIST"
cd ~
rm -rf "$TMPDIR"
