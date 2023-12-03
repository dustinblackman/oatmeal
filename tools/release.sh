#!/usr/bin/env bash

set -e

PROGDIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
cd "$PROGDIR/.."

# Check everything is clear for release, generate third party file.
docker ps >/dev/null || (echo "Docker is not running" && exit 1)
cargo gha goreleaser check
cargo check
cargo cmd lint
cargo cmd test
cargo cmd thirdparty

# Update change log.
export OM_VERSION=$(cat Cargo.toml | grep version | head -n1 | awk -F '"' '{print $2}')
cargo bin git-cliff -o CHANGELOG.md --tag "v$OM_VERSION"
cargo bin dprint fmt

# Add release commit
git add .
git commit -m "feat: Release v$OM_VERSION"
git tag -a "v$OM_VERSION" -m "v$OM_VERSION"

# Update readme, build completions.
cargo build
cargo cmd build-completions
cargo xtask update-readme
cargo bin dprint fmt

# Override release commit with updated readme.
git add .
git commit --amend -m "feat: Release v$OM_VERSION"
git tag -d "v$OM_VERSION"
git tag -a "v$OM_VERSION" -m "v$OM_VERSION"
git push
git push --tags
sleep 2

# Trigger binary builds, wait for completion.
gh workflow run release.yml --ref main
sleep 5
while true; do
	res=$(cargo gha gh run list -R dustinblackman/oatmeal -w build --json conclusion,databaseId | jq -rc '.[0]')
	echo "Status: $res"
	if (echo "$res" | grep -q "success"); then
		break
	fi
	sleep 30
done

# Download binary builds and debug symbols.
rm -rf dist-gh
mkdir dist-gh
export GH_RUN_ID=$(cargo gha gh run list -R dustinblackman/oatmeal -w build --json databaseId | jq -rc '.[0].databaseId')
cargo gha gh run download -D dist-gh "$GH_RUN_ID"
fd -t f . './dist-gh' | grep -v -i -E '(dwp|dSYM|pdb)' | xargs -L1 chmod +x

# Archive and upload debug packages
tar --strip-components=2 -czf "dist/oatmeal-DEBUG-${OM_VERSION}_darwin_arm64.tar.gz" dist-gh/aarch64-apple-darwin/oatmeal.dSYM
tar --strip-components=2 -czf "dist/oatmeal-DEBUG-${OM_VERSION}_windows_arm64.tar.gz" dist-gh/aarch64-pc-windows-msvc/oatmeal.pdb
tar --strip-components=2 -czf "dist/oatmeal-DEBUG-${OM_VERSION}_linux_arm64.tar.gz" dist-gh/aarch64-unknown-linux-gnu/oatmeal.dwp
tar --strip-components=2 -czf "dist/oatmeal-DEBUG-${OM_VERSION}_darwin_amd64.tar.gz" dist-gh/x86_64-apple-darwin/oatmeal.dSYM
tar --strip-components=2 -czf "dist/oatmeal-DEBUG-${OM_VERSION}_windows_amd64.tar.gz" dist-gh/x86_64-pc-windows-msvc/oatmeal.pdb
tar --strip-components=2 -czf "dist/oatmeal-DEBUG-${OM_VERSION}_linux_amd64.tar.gz" dist-gh/x86_64-unknown-linux-gnu/oatmeal.dwp
ls dist | grep DEBUG | while read f; do cargo gha gh release upload "v$OM_VERSION" "dist/$f"; done

# Release to Github
AUR_KEY=$(cat ~/.ssh/aur) cargo gha goreleaser --clean
cargo bin git-cliff --latest --strip header | cargo bin dprint fmt --stdin md | cargo gha gh release edit "v$OM_VERSION" --notes-file -

# Release to package managers not supported by GoReleaser.
cargo publish --no-verify
tools/apt.sh "$OM_VERSION" "$(realpath dist)"
tools/nur.sh "$OM_VERSION" "$(realpath dist)"
tools/yum.sh "$OM_VERSION" "$(realpath dist)"
tools/choco.sh "$OM_VERSION" "$(realpath dist)"
