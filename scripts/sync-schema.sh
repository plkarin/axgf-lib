#!/usr/bin/env bash
# Download the AXGF JSON Schemas from the canonical axgf-spec repository and
# refresh the vendored copies under schema/: axgf-1.0.schema.json and
# axgf-1.1.schema.json.
#
# The vendored copies are embedded at compile time via include_str! so the
# library can validate bundles offline (and in WASM, where fetching is
# impossible).
#
# This script is the ONLY supported way to update the vendored schemas.
# Never edit schema/axgf-*.schema.json by hand.

set -euo pipefail

VERSIONS=("1.0" "1.1")
BASE_URL="https://raw.githubusercontent.com/plkarin/axgf-spec/main/schema"
DEST_DIR="$(dirname "$0")/../schema"

for v in "${VERSIONS[@]}"; do
    url="$BASE_URL/axgf-$v.schema.json"
    dest="$DEST_DIR/axgf-$v.schema.json"
    tmp="$(mktemp)"
    trap 'rm -f "$tmp"' EXIT

    echo "Downloading $url"
    if ! curl --fail --silent --show-error --location --output "$tmp" "$url"; then
        echo "ERROR: download failed; vendored schema $v left unchanged" >&2
        exit 1
    fi

    if ! python3 -c "import json,sys; json.load(open(sys.argv[1]))" "$tmp" >/dev/null 2>&1; then
        echo "ERROR: downloaded file is not valid JSON; vendored schema $v left unchanged" >&2
        exit 1
    fi

    mv "$tmp" "$dest"
    trap - EXIT

    sha="$(sha256sum "$dest" | awk '{print $1}')"
    echo "Wrote $dest"
    echo "sha256: $sha"
done
