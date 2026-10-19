#!/usr/bin/env python3
"""Generate crates/world-geomag/src/coefficients.rs from a WMM.COF file.

Usage (stdlib only):
    python3 scripts/generate_wmm_coefficients.py crates/world-geomag/data/WMM.COF \
        > crates/world-geomag/src/coefficients.rs

WMM.COF is the coefficient file of the official World Magnetic Model
release (https://www.ncei.noaa.gov/products/world-magnetic-model). CI
regenerates the module from the committed file and fails on any
difference, and checks the file's sha256 against the provenance that
world_geomag::WMM2025 records.
"""

import sys

with open(sys.argv[1], "rb") as source:
    lines = source.read().decode().splitlines()
rows = []
for line in lines[1:]:
    fields = line.split()
    if len(fields) != 6:
        break
    rows.append(fields)

out = [
    "//! WMM2025 Gauss coefficients, as published.",
    "//!",
    "//! Transcribed from the coefficient file [`crate::WMM2025`]'s provenance",
    "//! names; that provenance carries the file's checksum.",
    "",
    "/// Maximum spherical-harmonic degree.",
    "pub(crate) const MAX_DEGREE: usize = %s;" % rows[-1][0],
    "",
    "/// `(n, m, g, h, g_dot, h_dot)`: Schmidt semi-normalized coefficients in",
    "/// nT and their secular variation in nT/year.",
    "#[rustfmt::skip]",
    "pub(crate) const COEFFICIENTS: [(usize, usize, f64, f64, f64, f64); %d] = [" % len(rows),
]
out += ["    (%s, %s, %s, %s, %s, %s)," % tuple(row) for row in rows]
out.append("];")
print("\n".join(out))
