"""Runs every case through an oracle binary, one fresh process per
evaluation, and writes `D(1..9) T(1) T(2) GTD7D-D(6)` per line."""
import subprocess
import sys

binary, style, cases_path = sys.argv[1:4]
for case in open(cases_path).read().splitlines():
    rows = []
    for mode in ("gtd7", "gtd7d"):
        if style == "c":
            run = subprocess.run([binary, mode], input=case, capture_output=True, text=True, check=True)
        else:
            flag = "1 " if mode == "gtd7d" else "0 "
            run = subprocess.run([binary], input=flag + case + "\n", capture_output=True, text=True, check=True)
        rows.append(run.stdout.split())
    assert len(rows[0]) == 11 and len(rows[1]) == 11, (case, rows)
    print(" ".join(rows[0] + [rows[1][5]]))
