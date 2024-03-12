#!/usr/bin/env bash

set -e

PROGDIR=$(dirname "$(readlink -f "$0")")
cd "$PROGDIR/.."

BUILD_ARCH="$1"
export EMPTY_GITHUB_TOKEN="$2"

(
	cd tools/node
	npm ci
)

echo "$EMPTY_GITHUB_TOKEN" | gh auth login --with-token
sudo apt-get update
sudo apt-get install gcc-aarch64-linux-gnu
rustup target add aarch64-unknown-linux-gnu
cargo install cargo-run-bin
cargo binstall --help
export CARGO_BUILD_TARGET=aarch64-unknown-linux-gnu
cargo bin committed --help || echo ""
cargo bin mise --help || echo ""
cargo cmd --help || echo ""
cargo nextest --help || echo ""
cargo insta --help || echo ""
cargo deny --helpA || echo ""
cargo watch --help || echo ""
find .bin

# echo "[BUILD] Building $BUILD_ARCH"
# tools/node/node_modules/.bin/devcontainer build --workspace-folder . --config ./.devcontainer/devcontainer-src.json --push --platform "linux/$BUILD_ARCH" --image-name ghcr.io/dustinblackman/devcontainer-oatmeal:latest-"$BUILD_ARCH"
