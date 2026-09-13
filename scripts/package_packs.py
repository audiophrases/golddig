#!/usr/bin/env python3
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
"""Packages built dictionary packs for distribution.

    python scripts/package_packs.py                 # every pack in packs/
    python scripts/package_packs.py ca es           # by language code
    python scripts/package_packs.py --out release/  # somewhere else

Each pack is gzipped (whole-file, which compresses ~5–6x; see docs/benchmarks) and written
to the output directory alongside SHA256SUMS and a short README, so the set can be attached
to a GitHub Release as-is. On another machine, scripts/install_packs.py verifies the
checksum and unpacks into the app's data directory.

Packs are gzipped whole rather than per-row compressed on disk: an already-compressed
pack barely compresses again for transport, so per-row compression would shrink the
installed footprint at the cost of a larger download. This project optimises for download.
"""

import gzip
import hashlib
import os
import shutil
import sqlite3
import sys
import time

CODE_TO_FILE = {
    "en": "kaikki-english.sqlite",
    "ca": "kaikki-catalan.sqlite",
    "es": "kaikki-spanish.sqlite",
    "fr": "kaikki-french.sqlite",
    "de": "kaikki-german.sqlite",
    "ary": "kaikki-darija.sqlite",
    "zh": "kaikki-chinese.sqlite",
}


def sha256_of(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def pack_summary(path: str) -> tuple[str, int, list[str]]:
    """(pack id, entry count, languages) read from the pack itself."""
    import json

    conn = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    try:
        manifest = json.loads(
            conn.execute("SELECT value FROM manifest WHERE key = 'manifest'").fetchone()[0]
        )
        entries = conn.execute("SELECT count(*) FROM entries").fetchone()[0]
        return manifest.get("id", "?"), entries, manifest.get("languages", [])
    finally:
        conn.close()


def main() -> None:
    args = sys.argv[1:]
    out_dir = "release"
    if "--out" in args:
        i = args.index("--out")
        out_dir = args[i + 1]
        del args[i : i + 2]

    if args:
        wanted = []
        for code in args:
            if code not in CODE_TO_FILE:
                print(f"unknown language code {code!r}; known: {', '.join(CODE_TO_FILE)}")
                sys.exit(1)
            wanted.append(CODE_TO_FILE[code])
    else:
        wanted = sorted(
            f for f in os.listdir("packs") if f.endswith(".sqlite") and f != "vertical-slice.sqlite"
        )

    os.makedirs(out_dir, exist_ok=True)
    sums = []
    rows = []
    for name in wanted:
        src = os.path.join("packs", name)
        if not os.path.exists(src):
            print(f"[skip] {src} not built — run scripts/build_all_packs.py first")
            continue

        pack_id, entries, languages = pack_summary(src)
        dst = os.path.join(out_dir, name + ".gz")
        raw = os.path.getsize(src)
        t0 = time.time()
        print(f"[gzip] {name} ({raw/1048576:,.1f} MB) ...", end="", flush=True)
        with open(src, "rb") as fin, gzip.open(dst, "wb", compresslevel=6) as fout:
            shutil.copyfileobj(fin, fout, 1 << 20)
        packed = os.path.getsize(dst)
        print(f" {packed/1048576:,.1f} MB ({raw/packed:.1f}x) in {time.time()-t0:.0f}s")

        digest = sha256_of(dst)
        sums.append(f"{digest}  {name}.gz")
        rows.append((name + ".gz", pack_id, ",".join(languages), entries, raw, packed))

    if not rows:
        print("nothing packaged")
        sys.exit(1)

    with open(os.path.join(out_dir, "SHA256SUMS"), "w", encoding="utf-8") as fh:
        fh.write("\n".join(sums) + "\n")

    total_raw = sum(r[4] for r in rows)
    total_packed = sum(r[5] for r in rows)
    with open(os.path.join(out_dir, "README.md"), "w", encoding="utf-8") as fh:
        fh.write("# Golddig dictionary packs\n\n")
        fh.write("Built from Wiktionary via Wiktextract/Kaikki (CC BY-SA 4.0). See NOTICE.md in the\n")
        fh.write("Golddig repository for attribution.\n\n")
        fh.write("Install with `python scripts/install_packs.py <file.sqlite.gz> ...` from a Golddig\n")
        fh.write("checkout, or gunzip and copy the .sqlite into the directory that `golddig --pack-dirs`\n")
        fh.write("reports. Verify downloads against SHA256SUMS.\n\n")
        fh.write("| File | Pack | Languages | Entries | Unpacked | Download |\n")
        fh.write("| --- | --- | --- | ---: | ---: | ---: |\n")
        for f, pid, langs, n, raw, packed in rows:
            fh.write(f"| `{f}` | {pid} | {langs} | {n:,} | {raw/1048576:,.0f} MB | {packed/1048576:,.0f} MB |\n")
        fh.write(f"| **total** | | | | **{total_raw/1048576:,.0f} MB** | **{total_packed/1048576:,.0f} MB** |\n")

    print(f"\n{len(rows)} pack(s) -> {out_dir}/")
    print(f"  unpacked {total_raw/1048576:,.0f} MB, download {total_packed/1048576:,.0f} MB")
    print(f"  SHA256SUMS and README.md written")


if __name__ == "__main__":
    main()
