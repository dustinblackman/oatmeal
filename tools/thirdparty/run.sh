#!/usr/bin/env bash

set -e

PROGDIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

APPROVED_LICENSE_OVERRIDES="$(cat "$PROGDIR"/approved_licenses.json | jq -rc 'to_entries[] | .value[]' | sd '\n' '|')ignoremeplz"
LICENSES=$(cargo about generate -c "$PROGDIR/about.toml" "$PROGDIR/templates/json-nl.hbs")

function failLint() {
	touch "$PROGDIR/.failed-lint"
}

function grepBadStrings() {
	if (echo "$LICENSES" | grep -v -E "($APPROVED_LICENSE_OVERRIDES)" | grep -q "$1"); then
		pkgs=$(echo "$LICENSES" | grep "$1" | jq -rc '"\(.package_name_version) - \(.license) - \(.link)"')

		echo "ERROR: Bad license found grepping for text: $1"
		echo -e "This effects the following packages:\n$pkgs"
		echo ""

		failLint
	fi
}

rm -f "$PROGDIR/.failed-lint"

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
		if (!(echo "$LICENSES" | jq -rc '.package_name_version' | grep -q "^${f}--")); then
			echo "ERROR: Missing license $f for target $target"
			failLint
		fi
	done
done

# Verify there is no unused clarifications in about.toml
cat "$PROGDIR/about.toml" | grep 'clarify' | sd '\[' '' | awk -F '.' '{print $1}' | sort | uniq | while read f; do
	if (!(cat "$PROGDIR/../../Cargo.lock" | grep -q "$f")); then
		echo "ERROR: about.toml has unused clarification for $f"
		failLint
	fi
done

# Verify there are no approved licenses that are no longer used.
cat "$PROGDIR"/approved_licenses.json | jq -rc 'to_entries[] | .value[]' | while read f; do
	if (!(echo "$LICENSES" | grep -q "$f")); then
		echo "ERROR: approved_licenses.json has unused approval for $f"
		failLint
	fi
done

if [ -f "$PROGDIR/.failed-lint" ]; then
	exit 1
fi

# Cargo packages
cargo about generate -c "$PROGDIR/about.toml" "$PROGDIR/templates/html.hbs" >"$PROGDIR/../../THIRDPARTY.html"

# Additional Third Party
sd '__BETTERTLS_LICENSE__' "$(curl -s -L https://raw.githubusercontent.com/rustls/webpki/4a39e2b67d4cddf58b0ea16dd821a04ee2240058/third-party/bettertls/LICENSE)" "$PROGDIR/../../THIRDPARTY.html"
sd '__CHROMIUM_LICENSE__' "$(curl -s -L https://raw.githubusercontent.com/rustls/webpki/7f0632ba67f99292600d8d47ea6f898bd72a4e8a/third-party/chromium/LICENSE)" "$PROGDIR/../../THIRDPARTY.html"

# Themes
sd '__BASE16_TEXTMATE__' "$(cat "$PROGDIR"/../../.cache/themes/chriskempson-base16-textmate/LICENSE.md | awk 'NR > 1')" "$PROGDIR/../../THIRDPARTY.html"

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
