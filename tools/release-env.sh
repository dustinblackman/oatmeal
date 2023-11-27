#!/usr/bin/env bash

export PATHTMP=$(echo "$PATH" | sd ':' '\n' | grep -E '(^/usr|^/bin|homebrew)' | grep -v "$USER" | sd '\n' ':')
export HOMETMP="$TMPDIR/homedir-$(basename "$PWD")"

rm -rf "$HOMETMP"
mkdir -p "$HOMETMP"
ln -s "$HOME/.cargo" "$HOMETMP/.cargo"
ln -s "$HOME/.rustup" "$HOMETMP/.rustup"

env -i PATH="$HOMETMP/.cargo/bin:$PATHTMP" HOME="$HOMETMP" bash --noprofile --norc -c "cargo $*"

rm -rf "$HOMETMP"
