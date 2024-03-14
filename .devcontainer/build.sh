#!/usr/bin/env bash

set -e

PROGDIR=$(dirname "$(readlink -f "$0")")
cd "$PROGDIR/.."

BUILD_ARCH="$1"

(
	cd tools/node
	npm ci
)

echo "[BUILD] Building $BUILD_ARCH"
tools/node/node_modules/.bin/devcontainer build --workspace-folder . --config ./.devcontainer/devcontainer-src.json --push --platform "linux/$BUILD_ARCH" --image-name ghcr.io/dustinblackman/devcontainer-oatmeal:latest-"$BUILD_ARCH"
