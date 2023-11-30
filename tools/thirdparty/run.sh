#!/usr/bin/env bash

set -e

PROGDIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
APPROVED_DEFAULT_LICENSES="$(cat "$PROGDIR"/approved_licenses.json | jq -rc 'to_entries[] | .value[]' | sd '\n' '|')ignoremeplz"
LICENSES=$(cargo about generate -c "$PROGDIR/about.toml" "$PROGDIR/templates/json-nl.hbs" | grep -v -E "($APPROVED_DEFAULT_LICENSES)")

FAILED_LINT="false"

function grepBadStrings() {
	if (echo "$LICENSES" | grep -q "$1"); then
		pkgs=$(echo "$LICENSES" | grep "$1" | jq -rc '"\(.package_name_version) - \(.license) - \(.link)"')

		echo "ERROR: Bad license found grepping for text: $1"
		echo -e "This effects the following packages:\n$pkgs"
		echo ""

		FAILED_LINT="true"
	fi
}

# Lint first
grepBadStrings "Copyright (c) <year> <copyright holders>"
grepBadStrings "LICENSE-APACHE or"
grepBadStrings "LICENSE-MIT or"

# Manually verify all Apache licenses.
grepBadStrings "apache"
grepBadStrings "Apache"

if [[ "$FAILED_LINT" == "true" ]]; then
	exit 1
fi

# Cargo packages
cargo about generate -c "$PROGDIR/about.toml" "$PROGDIR/templates/html.hbs" >"$PROGDIR/../../THIRDPARTY.html"

# Additional Third Party
sd '__OPENSSL_LICENSE__' "$(curl -s -L https://raw.githubusercontent.com/sfackler/rust-openssl/48b8e81e807b6f589ae66fed9567084c3d833b83/THIRD_PARTY)" "$PROGDIR/../../THIRDPARTY.html"

# Themes
sd '__BASE16_TEXTMATE__' "$(cat "$PROGDIR"/../../.cache/themes/LICENSE.md | awk 'NR > 1')" "$PROGDIR/../../THIRDPARTY.html"

# Syntaxes
SYNTAXES=$(find "$PROGDIR"/../../.cache/syntaxes | grep -i LICENSE | sort | while read f; do
	project=$(basename "$(dirname "$f")")
	text=$(cat "$f" | python3 -c 'import html, sys; [print(html.escape(l), end="") for l in sys.stdin]')
	license="MIT License"

	if (cat "$f" | grep -q -i "apache"); then
		license="Apache 2.0 License"
	fi

	if (cat "$f" | grep -q -i "fuck"); then
		license="Do What The Fuck You Want License"
	fi

	echo "<li class=\"license\"><h3>${license} - ${project}</h3><pre class=\"license-text\">${text}</pre></li>"
done)

sd '__EXTERNAL_SYNTAXES__' "$SYNTAXES" "$PROGDIR/../../THIRDPARTY.html"

# End
echo ""
