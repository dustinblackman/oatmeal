#!/usr/bin/env bash

set -e

PROGDIR=$(dirname "$(readlink -f "$0")")
cd "$PROGDIR/.."

COMMIT=$(git log --pretty=format:'%h' -n 1 .)

(
	cd tools/node
	npm ci
)

tools/node/node_modules/.bin/devcontainer build --workspace-folder . --config ./.devcontainer/devcontainer-src.json --push --platform "linux/arm64" --image-name ghcr.io/dustinblackman/devcontainer-oatmeal:latest-arm64
tools/node/node_modules/.bin/devcontainer build --workspace-folder . --config ./.devcontainer/devcontainer-src.json --push --platform "linux/amd64" --image-name ghcr.io/dustinblackman/devcontainer-oatmeal:latest-amd64
docker manifest create ghcr.io/dustinblackman/devcontainer-oatmeal:latest ghcr.io/dustinblackman/devcontainer-oatmeal:latest-arm64 ghcr.io/dustinblackman/devcontainer-oatmeal:latest-amd64
BUILD_SHA=$(docker manifest push ghcr.io/dustinblackman/devcontainer-oatmeal:latest)

DC_UPDATE=$(yq ".services.oatmeal.image = \"ghcr.io/dustinblackman/devcontainer-oatmeal@${BUILD_SHA}\"" ./.devcontainer/docker-compose.yml)
rm ./.devcontainer/docker-compose.yml
echo "$DC_UPDATE" >./.devcontainer/docker-compose.yml

git config --global user.email github-actions[bot]@users.noreply.github.com
git config --global user.name github-actions[bot]
git config pull.rebase false
BUILD_BRANCH="devcontainer-image-update-$GITHUB_RUN_ID"
git checkout -b "$BUILD_BRANCH"
git add ./.devcontainer/docker-compose.yml
git commit -m 'chore: Update dev container image sha'
git push origin "$BUILD_BRANCH"
gh pr create --title "[CI] Update devcontainer SHA to $BUILD_SHA" --body "Updates the dev container base imaged based on the changes made in https://github.com/dustinblackman/oatmeal/commit/${GITHUB_SHA}"
