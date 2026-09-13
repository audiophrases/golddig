#!/usr/bin/env python3
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
"""Installs downloaded dictionary packs on this machine.

    python scripts/install_packs.py kaikki-catalan.sqlite.gz kaikki-spanish.sqlite.gz
    python scripts/install_packs.py --dir /path/to/release   # every .sqlite.gz there
    python scripts/install_packs.py --to C:/somewhere/packs   # override the target

Verifies each file against a SHA256SUMS found beside it (skipped, with a warning, when
there is none), decompresses into the per-user pack directory Golddig reads at startup,
and writes to a temporary name first so an interrupted install never leaves a truncated
pack behind for the app to trip over.

The target directory is the same one `golddig --pack-dirs` reports as the per-user
location. It is computed here without needing the app installed, so packs can be put in
place before the first launch.
"""

import gzip
import hashlib
import os
import shutil
import sqlite3
import sys

APP_IDENTIFIER = "com.golddig.app"


def default_pack_dir() -> str:
    """Mirrors Tauri's app_data_dir() for this app's identifier, per platform."""
    if sys.platform.startswith("win"):
        base = os.environ.get("APPDATA") or os.path.expanduser(r"~\AppData\Roaming")
    elif sys.platform == "darwin":
        base = os.path.expanduser("~/Library/Application Support")
    else:
        base = os.environ.get("XDG_DATA_HOME") or os.path.expanduser("~/.local/share")
    return os.path.join(base, APP_IDENTIFIER, "packs")


def sha256_of(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def expected_sums(directory: str) -> dict[str, str]:
    path = os.path.join(directory, "SHA256SUMS")
    if not os.path.exists(path):
        return {}
    sums = {}
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            parts = line.split()
            if len(parts) == 2:
                sums[parts[1]] = parts[0]
    return sums


def verify_pack(path: str) -> str:
    """Opens the unpacked pack read-only and returns a one-line description, or raises."""
    import json

    conn = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    try:
        ok = conn.execute("PRAGMA quick_check").fetchone()[0]
        if ok != "ok":
            raise RuntimeError(f"integrity check failed: {ok}")
        manifest = json.loads(
            conn.execute("SELECT value FROM manifest WHERE key = 'manifest'").fetchone()[0]
        )
        entries = conn.execute("SELECT count(*) FROM entries").fetchone()[0]
        return f"{manifest.get('name', '?')} — {entries:,} entries"
    finally:
        conn.close()


def install_one(src: str, target_dir: str, sums: dict[str, str]) -> bool:
    name = os.path.basename(src)
    if not name.endswith(".sqlite.gz"):
        print(f"[skip] {name}: not a .sqlite.gz pack")
        return False

    expected = sums.get(name)
    if expected:
        actual = sha256_of(src)
        if actual != expected:
            print(f"[FAIL] {name}: checksum mismatch — download is corrupt or tampered")
            return False
        print(f"[ok]   {name}: checksum verified")
    else:
        print(f"[warn] {name}: no SHA256SUMS beside it, installing unverified")

    final = os.path.join(target_dir, name[: -len(".gz")])
    partial = final + ".part"
    print(f"       unpacking to {final} ...", end="", flush=True)
    with gzip.open(src, "rb") as fin, open(partial, "wb") as fout:
        shutil.copyfileobj(fin, fout, 1 << 20)
    try:
        description = verify_pack(partial)
    except Exception as err:
        os.remove(partial)
        print(f"\n[FAIL] {name}: unpacked file is not a valid pack ({err})")
        return False
    os.replace(partial, final)
    print(f" {os.path.getsize(final)/1048576:,.0f} MB")
    print(f"       {description}")
    return True


def main() -> None:
    args = sys.argv[1:]
    target_dir = default_pack_dir()
    if "--to" in args:
        i = args.index("--to")
        target_dir = args[i + 1]
        del args[i : i + 2]

    files: list[str] = []
    if "--dir" in args:
        i = args.index("--dir")
        directory = args[i + 1]
        del args[i : i + 2]
        files += [
            os.path.join(directory, f)
            for f in sorted(os.listdir(directory))
            if f.endswith(".sqlite.gz")
        ]
    files += args

    if not files:
        print(__doc__)
        sys.exit(1)

    os.makedirs(target_dir, exist_ok=True)
    print(f"Installing into {target_dir}\n")

    installed = 0
    for src in files:
        if not os.path.exists(src):
            print(f"[skip] {src}: not found")
            continue
        if install_one(src, target_dir, expected_sums(os.path.dirname(src) or ".")):
            installed += 1

    print(f"\n{installed} of {len(files)} pack(s) installed. Restart Golddig to load them.")
    sys.exit(0 if installed == len(files) else 1)


if __name__ == "__main__":
    main()
