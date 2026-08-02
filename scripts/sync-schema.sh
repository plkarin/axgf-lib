#!/usr/bin/env bash
# Download the AXGF 1.0 JSON Schema from the canonical axgf-spec repository
# and refresh the vendored copy at schema/axgf-1.0.schema.json.
#
# The vendored copy is embedded at compile time via include_str! so the
# library can validate bundles offline (and in WASM, where fetching is
# impossible).
#
# This script is the ONLY supported way to update the vendored schema.
# Never edit schema/axgf-1.0.schema.json by hand.

set -euo pipefail

UPSTREAM_URL="https://raw.githubusercontent.com/plkarin/axgf-spec/main/schema/axgf-1.0.schema.json"
DEST="$(dirname "$0")/../schema/axgf-1.0.schema.json"

TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

echo "Downloading $UPSTREAM_URL"
if ! curl --fail --silent --show-error --location --output "$TMP" "$UPSTREAM_URL"; then
    echo "ERROR: download failed; vendored schema left unchanged" >&2
    exit 1
fi

if ! python3 -c "import json,sys; json.load(open(sys.argv[1]))" "$TMP" >/dev/null 2>&1; then
    echo "ERROR: downloaded file is not valid JSON; vendored schema left unchanged" >&2
    exit 1
fi

mv "$TMP" "$DEST"
trap - EXIT

SHA="$(sha256sum "$DEST" | awk '{print $1}')"
echo "Wrote $DEST"
echo "sha256: $SHA"
