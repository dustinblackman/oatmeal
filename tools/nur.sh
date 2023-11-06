#!/usr/bin/env bash

set -e

VERSION="$1"
DIST="$2"
TMPDIR="$(mktemp -d)"

cd "$TMPDIR"
git clone --depth 1 git@github.com:dustinblackman/nur-packages.git .
./scripts/goreleaser.sh oatmeal "$VERSION" "${DIST}/oatmeal_${VERSION}_linux_amd64.tar.gz"
cd ~
rm -rf "$TMPDIR"
