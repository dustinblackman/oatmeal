#!/usr/bin/env bash

set -e

PROGDIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
APPROVED_DEFAULT_LICENSES="$(cat "$PROGDIR"/approved_licenses.json | jq -rc 'to_entries[] | .value[]' | sd '\n' '|')ignoremeplz"
LICENSES=$(cargo about generate -c "$PROGDIR/about.toml" "$PROGDIR/templates/json-nl.hbs")

FAILED_LINT="false"

function grepBadStrings() {
	if (echo "$LICENSES" | grep -v -E "($APPROVED_DEFAULT_LICENSES)" | grep -q "$1"); then
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

# Final sanity check that all cargo packages are in there.
yq -p=toml -o=json "$PROGDIR/about.toml" | jq -rc '.targets[]' | while read target; do
	cargo tree --target "$target" -e normal -f '{p}' | sd ' ' '\n' | grep '[a-zA-Z]' | grep -v '^v\d' | grep -v -E "(oatmeal|.git|(proc-macro))" | sort | uniq | while read f; do
		echo "$LICENSES" | jq -rc '.package_name_version' | grep -q "^${f}--" || (echo "ERROR: Missing license $f for target $target" && FAILED_LINT="true")
	done
done

# Verify there is no unused clarifications in about.toml
cat "$PROGDIR/about.toml" | grep 'clarify' | sd '\[' '' | awk -F '.' '{print $1}' | sort | uniq | while read f; do
	cat "$PROGDIR/../../Cargo.lock" | grep -q "$f" || (echo "ERROR: about.toml has unused clarification for $f" && FAILED_LINT="true")
done

if [[ "$FAILED_LINT" == "true" ]]; then
	exit 1
fi

# Cargo packages
cargo about generate -c "$PROGDIR/about.toml" "$PROGDIR/templates/html.hbs" >"$PROGDIR/../../THIRDPARTY.html"

# Additional Third Party
sd '__BETTERTLS_LICENSE__' "$(curl -s -L https://raw.githubusercontent.com/rustls/webpki/v/0.101.7/third-party/bettertls/LICENSE)" "$PROGDIR/../../THIRDPARTY.html"
sd '__CHROMIUM_LICENSE__' "$(curl -s -L https://raw.githubusercontent.com/rustls/webpki/v/0.101.7/third-party/chromium/LICENSE)" "$PROGDIR/../../THIRDPARTY.html"

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
