#!/bin/bash

set -e

build="$1"
target="$2"

goTargetToRust() {
	if [[ "$build" == "musl" ]]; then
		if [[ "$target" == "linux_amd64_v1" ]]; then
			echo "x86_64-unknown-linux-musl"
		elif [[ "$target" == "linux_arm64" ]]; then
			echo "aarch64-unknown-linux-musl"
		else
			echo "goreleaser-dist.sh is not prepared to handle builds for ${target}. Please update script."
			exit 1
		fi
	else
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
	fi
}

rm -rf "./dist/${build}_${target}"
mkdir -p "./dist/${build}_${target}"

rustbin="./dist-gh/$(goTargetToRust)/oatmeal"
if [[ "$target" == "windows_amd64_v1" ]] || [[ "$target" == "windows_arm64" ]]; then
	rustbin="${rustbin}.exe"
fi

cp "$rustbin" "./dist/${build}_${target}/"
