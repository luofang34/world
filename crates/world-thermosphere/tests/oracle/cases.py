"""Writes the NRLMSISE-00 oracle case list: one evaluation per line."""
import random
import sys

rng = random.Random(20261124)
ALTS = [0, 5, 10, 20, 32.4999, 32.5, 32.5001, 45, 55, 62.5, 70, 72.4999, 72.5,
        72.5001, 85, 100, 110, 119.999, 120, 120.001, 150, 160, 200, 240, 250,
        300, 320, 400, 450, 500, 800, 1000, 2000]
LATS = [-90, -67.5, -45, -10, 0, 30, 60, 78.5, 90]
LONS = [-180, -70, 0, 15, 120.5, 359]
DOYS = [1, 81, 172, 265, 300, 366]
SECS = [0, 29000, 43200, 75000, 86399]
FLUX = [(70, 70), (150, 150), (150, 180), (120, 95), (220, 250), (300, 65)]
APS = [0, 4, 15, 40, 100, 300]
HISTS = [
    [100, 100, 100, 100, 100, 100, 100],
    [4, 4, 4, 4, 4, 4, 4],
    [48, 300, 207, 94, 39, 12, 5],
    [15, 3, 7, 22, 56, 80, 9],
    [0, 0, 0, 0, 0, 0, 0],
]
ON = [1] * 23


def line(alt, hist=None, switches=ON):
    doy = rng.choice(DOYS)
    sec = rng.choice(SECS)
    lat = rng.choice(LATS)
    lon = rng.choice(LONS)
    lst = rng.choice([4, 16, 23.5, round(sec / 3600 + lon / 15, 6)])
    f107a, f107 = rng.choice(FLUX)
    if hist is None:
        hist = rng.random() < 0.2
    ap = rng.choice(HISTS) if hist else [rng.choice(APS)] + [0] * 6
    fields = [doy, sec, alt, lat, lon, lst, f107a, f107, int(hist)] + ap + list(switches)
    return " ".join(repr(v) for v in fields)


out = []
for alt in ALTS:
    for _ in range(5):
        out.append(line(alt))
for alt in rng.sample(ALTS, 20):
    out.append(line(alt, hist=True))
for effect in range(1, 24):
    for setting in (0, 2):
        switches = list(ON)
        switches[effect - 1] = setting
        alts = [300, 95] if setting == 0 else [300]
        # Below 72.5 km the references read a value that switch 15 leaves unset.
        if effect != 15:
            alts.append(40)
        for alt in alts:
            out.append(line(alt, hist=(len(out) % 3 == 0), switches=switches))
sys.stdout.write("\n".join(out) + "\n")
