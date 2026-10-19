#!/usr/bin/env bash
# The WMM2025 coefficient table must be exactly what the committed
# WMM.COF generates, and that file must be the one WMM2025's provenance
# names by checksum. --self-test proves both checks can fail.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"

check() {
    local cof="$1" table="$2" field="$3"
    local recorded actual
    recorded="$(grep -o 'source_sha256: "[0-9a-f]*"' "$field" | grep -o '[0-9a-f]\{64\}')"
    actual="$(sha256sum "$cof" | cut -d' ' -f1)"
    if [[ "$recorded" != "$actual" ]]; then
        echo "WMM.COF sha256 $actual does not match the recorded provenance $recorded"
        return 1
    fi
    if ! python3 "$root/scripts/generate_wmm_coefficients.py" "$cof" | diff -q - "$table" >/dev/null; then
        echo "$table is not what $cof generates"
        return 1
    fi
}

geomag="$root/crates/world-geomag"
cof="$geomag/data/WMM.COF"
table="$geomag/src/coefficients.rs"
field="$geomag/src/field.rs"

if [[ "${1:-}" == "--self-test" ]]; then
    scratch="$(mktemp -d)"
    trap 'rm -rf "$scratch"' EXIT
    sed 's/(1, 0, -29351.8,/(1, 0, -29351.9,/' "$table" > "$scratch/table.rs"
    if check "$cof" "$scratch/table.rs" "$field" >/dev/null; then
        echo "self-test: an edited table passed"; exit 1
    fi
    { cat "$cof"; echo; } > "$scratch/WMM.COF"
    if check "$scratch/WMM.COF" "$table" "$field" >/dev/null; then
        echo "self-test: a different coefficient file passed"; exit 1
    fi
    echo "self-test: both checks refuse"
    exit 0
fi

check "$cof" "$table" "$field"
echo "WMM2025 coefficients reproduce from the recorded WMM.COF"
