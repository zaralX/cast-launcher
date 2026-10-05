#!/usr/bin/env bash
# Changes the launcher version on Linux and macOS. All the logic lives in set-version.mjs next to it.
#
#   ./scripts/set-version.sh            show the current versions
#   ./scripts/set-version.sh 1.5.0      set a new one everywhere
#   ./scripts/set-version.sh 1.5.0 -n   show what would change without writing

set -euo pipefail

if ! command -v node >/dev/null 2>&1; then
    echo "Error: node not found: it is needed to build the frontend too, install Node.js" >&2
    exit 1
fi

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

exec node "$here/set-version.mjs" "$@"
