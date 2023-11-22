#!/bin/bash

set -e

target="$1"

goTargetToRust() {
	if [[ "$target" == "darwin_amd64_v1" ]]; then
		echo "x86_64-apple-darwin"
	elif [[ "$target" == "darwin_arm64" ]]; then
		echo "aarch64-apple-darwin"
	elif [[ "$target" == "linux_amd64_v1" ]]; then
		echo "x86_64-unknown-linux-gnu"
	elif [[ "$target" == "linux_arm64" ]]; then
		echo "aarch64-unknown-linux-gnu"
	elif [[ "$target" == "windows_amd64_v1" ]]; then
		echo "x86_64-pc-windows-msvc"
	elif [[ "$target" == "windows_arm64" ]]; then
		echo "aarch64-pc-windows-msvc"
	else
		echo "goreleaser-dist.sh is not prepared to handle builds for ${target}. Please update script."
		exit 1
	fi
}

rm -rf "./dist/oatmeal_${target}"
mkdir -p "./dist/oatmeal_${target}"

rustbin="./target/$(goTargetToRust)/release/oatmeal"
if [[ "$target" == "windows_amd64_v1" ]] || [[ "$target" == "windows_arm64" ]]; then
	rustbin="${rustbin}.exe"
fi

cp "$rustbin" "./dist/oatmeal_${target}/"
