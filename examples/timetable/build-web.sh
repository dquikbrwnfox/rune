#!/usr/bin/env bash
# Builds the web app with --opt and puts the CELCAT data next to it as celcat.json.
#   TIMETABLE_DATA=/path/to/celcat.json bash examples/timetable/build-web.sh [OUT_DIR]
# Run from the Rune workspace root; OUT_DIR defaults to target/web-tt. Without TIMETABLE_DATA the app
# still builds, but at runtime it falls back to the small built-in sample and says so.
set -euo pipefail
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$here/../.." && pwd)
out=${1:-target/web-tt}
cd "$root"
bash scripts/web-build.sh timetable --opt --out "$out"
if [[ -n ${TIMETABLE_DATA:-} && -f $TIMETABLE_DATA ]]; then
    cp "$TIMETABLE_DATA" "$out/celcat.json"
    echo "copied $TIMETABLE_DATA -> $out/celcat.json"
else
    echo "warning: TIMETABLE_DATA is not set to a file, so $out/celcat.json was not written;" >&2
    echo "         the app will show the offline sample data." >&2
fi
