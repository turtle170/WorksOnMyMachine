#!/usr/bin/env bash
# Download the newest python-build-standalone CPython ("install_only") for a
# target triple and unpack it into <dest>/python. Needs gh and GH_TOKEN.
set -euo pipefail
triple="$1"
dest="$2"
asset="$(gh api repos/astral-sh/python-build-standalone/releases/latest --jq \
  "[.assets[].name | select(test(\"^cpython-3\\\\.[0-9]+\\\\.[0-9]+\\\\+[0-9]+-${triple}-install_only\\\\.tar\\\\.gz\$\"))]
   | sort_by(capture(\"cpython-3\\\\.(?<m>[0-9]+)\\\\.(?<p>[0-9]+)\") | [(.m|tonumber),(.p|tonumber)]) | last")"
if [[ -z "$asset" || "$asset" == "null" ]]; then
  echo "no CPython build found for $triple" >&2
  exit 1
fi
echo "bundling $asset"
tag="$(gh api repos/astral-sh/python-build-standalone/releases/latest --jq .tag_name)"
gh release download "$tag" --repo astral-sh/python-build-standalone --pattern "$asset" --dir "$RUNNER_TEMP"
mkdir -p "$dest"
tar xzf "$RUNNER_TEMP/$asset" -C "$dest"
test -d "$dest/python"
