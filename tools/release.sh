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

# Update readme, build completions, manpages.
cargo build
cargo cmd build-completions
rm -rf manpages && mkdir manpages && ./target/debug/oatmeal manpages | gzip -c -9 >manpages/oatmeal.1.gz
cargo xtask update-readme
rm -f config.example.toml
./target/debug/oatmeal config default >config.example.toml
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

# Download binary builds
rm -rf dist-gh
mkdir dist-gh
export GH_RUN_ID=$(cargo gha gh run list -R dustinblackman/oatmeal -w build --json databaseId | jq -rc '.[0].databaseId')
cargo gha gh run download -D dist-gh "$GH_RUN_ID"
fd -t f . './dist-gh' | grep -v -i -E '(dwp|dSYM|pdb)' | xargs -L1 chmod +x

# Release to Github
echo "$GITHUB_TOKEN" | docker login ghcr.io -u dustinblackman --password-stdin
AUR_KEY=$(cat ~/.ssh/aur) RPM_KEY="$HOME/.gpg/yum-private.key" cargo gha goreleaser --clean
cargo bin git-cliff --latest --strip header | cargo bin dprint fmt --stdin md | cargo gha gh release edit "v$OM_VERSION" --notes-file -
cargo gha gh pr list -R microsoft/winget-pkgs -A dustinblackman --state open --json number | jq -rc '.[] | .number' | while read f; do open "https://github.com/microsoft/winget-pkgs/pull/$f"; done

# Release to package managers not supported by GoReleaser.
cargo publish
tools/apt.sh "$OM_VERSION" "$(realpath dist)"
tools/nur.sh "$OM_VERSION" "$(realpath dist)"
tools/yum.sh "$OM_VERSION" "$(realpath dist)"
tools/choco.sh "$OM_VERSION" "$(realpath dist)"

# Archive and upload debug packages
ls dist-gh | while read f; do cp LICENSE THIRDPARTY.html "dist-gh/$f/"; done
tar --strip-components=2 -czf "dist/DEBUG-${OM_VERSION}_darwin_arm64.tar.gz" dist-gh/aarch64-apple-darwin/
tar --strip-components=2 -czf "dist/DEBUG-${OM_VERSION}_windows_arm64.tar.gz" dist-gh/aarch64-pc-windows-msvc/
tar --strip-components=2 -czf "dist/DEBUG-${OM_VERSION}_linux_arm64.tar.gz" dist-gh/aarch64-unknown-linux-gnu/
tar --strip-components=2 -czf "dist/DEBUG-${OM_VERSION}_linux-musl_arm64.tar.gz" dist-gh/aarch64-unknown-linux-musl/
tar --strip-components=2 -czf "dist/DEBUG-${OM_VERSION}_darwin_amd64.tar.gz" dist-gh/x86_64-apple-darwin/
tar --strip-components=2 -czf "dist/DEBUG-${OM_VERSION}_windows_amd64.tar.gz" dist-gh/x86_64-pc-windows-msvc/
tar --strip-components=2 -czf "dist/DEBUG-${OM_VERSION}_linux_amd64.tar.gz" dist-gh/x86_64-unknown-linux-gnu/
tar --strip-components=2 -czf "dist/DEBUG-${OM_VERSION}_linux-musl_amd64.tar.gz" dist-gh/x86_64-unknown-linux-musl/
ls dist | grep DEBUG | while read f; do cargo gha gh release upload "v$OM_VERSION" "dist/$f"; done
