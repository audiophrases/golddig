#!/usr/bin/env python3
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""Builds every Golddig language pack in size order, smallest first.

Smallest-first so a pipeline problem surfaces on a 6 MB download rather than a 3 GB one.
Each target is independent: one failure is reported and the run continues.

    python scripts/build_all_packs.py            # all targets
    python scripts/build_all_packs.py ca es      # just these
"""

import subprocess
import sys
import time

# (target, approximate download size in MB) in ascending size order.
ORDER = [
    ("ary", 6),
    ("ca", 230),
    ("fr", 551),
    ("es", 989),
    ("de", 1027),
    ("zh", 1148),
    ("en", 3094),
]


def main() -> None:
    wanted = sys.argv[1:]
    targets = [t for t in ORDER if not wanted or t[0] in wanted]
    if wanted and not targets:
        print(f"No known targets in {wanted}")
        sys.exit(1)

    total_mb = sum(mb for _, mb in targets)
    print(f"Building {len(targets)} packs, ~{total_mb:,} MB to download.\n")

    results = []
    for target, mb in targets:
        print("=" * 72)
        print(f"=== {target}  (~{mb:,} MB)")
        print("=" * 72, flush=True)
        t0 = time.time()
        res = subprocess.run(
            [sys.executable, "scripts/fetch_and_build_pack.py", target]
        )
        elapsed = time.time() - t0
        results.append((target, res.returncode, elapsed))
        print(f"=== {target}: exit {res.returncode} in {elapsed/60:.1f} min\n", flush=True)

    print("=" * 72)
    print("SUMMARY")
    for target, code, elapsed in results:
        status = "ok" if code == 0 else f"FAILED ({code})"
        print(f"  {target:10s} {status:14s} {elapsed/60:6.1f} min")
    failures = [t for t, c, _ in results if c != 0]
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
