#!/usr/bin/env bash
# Смена версии лаунчера, Linux и macOS. Вся логика - в set-version.mjs рядом.
#
#   ./scripts/set-version.sh            показать текущие версии
#   ./scripts/set-version.sh 1.5.0      проставить новую везде
#   ./scripts/set-version.sh 1.5.0 -n   показать, что изменится, но не писать

set -euo pipefail

if ! command -v node >/dev/null 2>&1; then
    echo "Ошибка: не найден node - он нужен и для сборки фронта, поставьте Node.js" >&2
    exit 1
fi

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

exec node "$here/set-version.mjs" "$@"
