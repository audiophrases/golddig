#!/usr/bin/env python3
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""Fetch a Kaikki/Wiktextract dump and compile it into a Golddig SQLite pack.

Usage:
    python scripts/fetch_and_build_pack.py <target> [max_entries]

Targets: en, simple-en, ca, es, fr, de, ary, zh   (see SOURCES)

Three things this script deliberately does, because their absence shipped three
separate defects:

1. It rebuilds `golddig-pack` before every run. The shipped packs were compiled by a
   binary predating the collocation extractor, so a feature that worked in source was
   absent from every artifact.
2. It generates the manifest from SOURCES. Four of the seven documented targets had no
   manifest file at all, so the documented command downloaded hundreds of megabytes and
   then failed; and the English manifest claimed "English Wiktionary" for a Simple
   English Wiktionary dump.
3. It prints a coverage report and fails on an empty pack. Coverage was never measured,
   which is why packs shipped with 0% translations and 0% collocations.
"""

import hashlib
import json
import os
import subprocess
import sys
import time
import urllib.request

KAIKKI = "https://kaikki.org/dictionary"

# Wikimedia project text has been CC BY-SA 4.0 since 2023; Wiktionary is additionally
# available under the GFDL. Wiktextract's extraction does not change the content licence.
WIKT_LICENSE = "CC-BY-SA-4.0"

SOURCES = {
    # Full English Wiktionary (~3.1 GB). The previous pipeline pointed `en` at the
    # Simple English Wiktionary extract (~39 MB) while still calling it "English
    # Wiktionary" — which is why the shipped English pack had no translations at all.
    "en": {
        "name": "English Wiktionary",
        "url": f"{KAIKKI}/English/kaikki.org-dictionary-English.jsonl",
        "lang": "en",
        "edition": "English Wiktionary",
        "output": "packs/kaikki-english.sqlite",
    },
    # Kept as an explicit, honestly-labelled target: it is small and fast for testing.
    "simple-en": {
        "name": "Simple English Wiktionary",
        "url": "https://kaikki.org/simplewiktionary/English/kaikki.org-dictionary-English.jsonl",
        "lang": "en",
        "edition": "Simple English Wiktionary",
        "output": "packs/kaikki-simple-english.sqlite",
    },
    "ca": {
        "name": "Catalan",
        "url": f"{KAIKKI}/Catalan/kaikki.org-dictionary-Catalan.jsonl",
        "lang": "ca",
        "edition": "English Wiktionary (Catalan entries)",
        "output": "packs/kaikki-catalan.sqlite",
    },
    "es": {
        "name": "Spanish",
        "url": f"{KAIKKI}/Spanish/kaikki.org-dictionary-Spanish.jsonl",
        "lang": "es",
        "edition": "English Wiktionary (Spanish entries)",
        "output": "packs/kaikki-spanish.sqlite",
    },
    "fr": {
        "name": "French",
        "url": f"{KAIKKI}/French/kaikki.org-dictionary-French.jsonl",
        "lang": "fr",
        "edition": "English Wiktionary (French entries)",
        "output": "packs/kaikki-french.sqlite",
    },
    "de": {
        "name": "German",
        "url": f"{KAIKKI}/German/kaikki.org-dictionary-German.jsonl",
        "lang": "de",
        "edition": "English Wiktionary (German entries)",
        "output": "packs/kaikki-german.sqlite",
    },
    "ary": {
        "name": "Moroccan Arabic (Darija)",
        "url": f"{KAIKKI}/Moroccan%20Arabic/kaikki.org-dictionary-MoroccanArabic.jsonl",
        "lang": "ary",
        "edition": "English Wiktionary (Moroccan Arabic entries)",
        "output": "packs/kaikki-darija.sqlite",
    },
    # /dictionary/Chinese/ carries the Han-character headwords. /dictionary/Mandarin/ —
    # what the previous pipeline used — is the pinyin romanization index, which is why
    # the shipped "Mandarin" pack contained 6 CJK lemmas out of 72,909.
    "zh": {
        "name": "Chinese",
        "url": f"{KAIKKI}/Chinese/kaikki.org-dictionary-Chinese.jsonl",
        "lang": "zh",
        "edition": "English Wiktionary (Chinese entries)",
        "output": "packs/kaikki-chinese.sqlite",
    },
}

# Targets whose upstream edition is expected to carry translation tables.
#
# Only the English-language extracts do. English Wiktionary publishes a translations
# table on an *English* lemma (english -> ca/es/fr/de/zh/...); a Catalan or Spanish entry
# on English Wiktionary carries an English gloss but no translations array. Verified
# empirically: 0 of the first 40,000 Catalan records have one, at entry or sense level.
#
# So the English pack is what supplies the translations the UI shows. Translations
# *between* two non-English languages need a genuinely bilingual source — FreeDict TEI or
# Apertium bilingual dictionaries — which is tracked in docs/roadmap.md.
EXPECT_TRANSLATIONS = {"en"}


def raw_path(target: str) -> str:
    return os.path.join("fixtures", f"kaikki-{target}.jsonl")


def manifest_path(target: str) -> str:
    return os.path.join("fixtures", f"kaikki-{target}.manifest.json")


def sha256_file(path: str, limit_mb: int | None = None) -> str:
    """SHA-256 of the file, or of its first limit_mb for very large dumps."""
    h = hashlib.sha256()
    budget = None if limit_mb is None else limit_mb * 1024 * 1024
    with open(path, "rb") as fh:
        while True:
            chunk = fh.read(1024 * 1024)
            if not chunk:
                break
            h.update(chunk)
            if budget is not None:
                budget -= len(chunk)
                if budget <= 0:
                    break
    return h.hexdigest()


def http_last_modified(url: str) -> str | None:
    try:
        req = urllib.request.Request(url, method="HEAD")
        with urllib.request.urlopen(req, timeout=60) as resp:
            return resp.headers.get("Last-Modified")
    except Exception:
        return None


def download_file(url: str, dest: str) -> None:
    if os.path.exists(dest) and os.path.getsize(dest) > 1024:
        mb = os.path.getsize(dest) / (1024 * 1024)
        print(f"[cache] {dest} present ({mb:.1f} MB), skipping download.")
        return
    print(f"[fetch] {url}\n     -> {dest}")
    t0 = time.time()
    last = [0.0]

    def progress(blocks: int, block_size: int, total: int) -> None:
        now = time.time()
        if now - last[0] < 2.0:
            return
        last[0] = now
        done = blocks * block_size
        if total > 0:
            pct = 100.0 * done / total
            print(f"       {done/1e6:,.0f} / {total/1e6:,.0f} MB  ({pct:.1f}%)", flush=True)
        else:
            print(f"       {done/1e6:,.0f} MB", flush=True)

    tmp = dest + ".part"
    urllib.request.urlretrieve(url, tmp, reporthook=progress)
    os.replace(tmp, dest)
    mb = os.path.getsize(dest) / (1024 * 1024)
    print(f"[fetch] done in {time.time()-t0:.1f}s ({mb:.1f} MB).")


def write_manifest(target: str, cfg: dict, raw: str) -> str:
    """Generates the manifest from SOURCES so metadata always matches the real dump."""
    path = manifest_path(target)
    retrieved = http_last_modified(cfg["url"]) or time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    manifest = {
        "id": f"kaikki-{target}",
        "name": f"Kaikki {cfg['edition']} — {cfg['name']}",
        "version": time.strftime("%Y.%m.%d", time.gmtime()),
        "schema_version": 1,
        "languages": [cfg["lang"]],
        "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "license": WIKT_LICENSE,
        "sources": [
            {
                "id": "kaikki-wiktionary",
                "name": f"Kaikki.org Wiktextract — {cfg['edition']}",
                "url": cfg["url"],
                "license": WIKT_LICENSE,
                "attribution": (
                    f"{cfg['edition']} contributors, extracted via Wiktextract "
                    f"and published by Kaikki.org. Retrieved {retrieved}. "
                    f"Source SHA-256 (first 64 MB): {sha256_file(raw, limit_mb=64)}"
                ),
            }
        ],
    }
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(manifest, fh, ensure_ascii=False, indent=2)
        fh.write("\n")
    print(f"[manifest] wrote {path}")
    return path


def build_cli() -> str:
    """Always rebuild, so a pack can never be compiled by a stale binary again."""
    exe = "golddig-pack.exe" if os.name == "nt" else "golddig-pack"
    print("[cargo] building golddig-pack (release)...")
    res = subprocess.run(
        ["cargo", "build", "--release", "--manifest-path",
         os.path.join("src-tauri", "Cargo.toml"), "--bin", "golddig-pack"]
    )
    if res.returncode != 0:
        print("[error] could not build golddig-pack", file=sys.stderr)
        sys.exit(res.returncode)
    path = os.path.join("src-tauri", "target", "release", exe)
    if not os.path.exists(path):
        print(f"[error] {path} missing after build", file=sys.stderr)
        sys.exit(1)
    return path


def report_coverage(db_path: str, target: str) -> int:
    """Prints per-field coverage and fails on an empty pack."""
    import sqlite3

    conn = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
    total = conn.execute("SELECT count(*) FROM entries").fetchone()[0]
    if total == 0:
        print("[error] pack contains zero entries", file=sys.stderr)
        return 1

    fields = {"ipa": 0, "examples": 0, "translations": 0, "synonyms": 0, "collocations": 0}
    for (blob,) in conn.execute("SELECT data_json FROM entries"):
        entry = json.loads(blob)
        senses = entry.get("senses") or []
        if any(p.get("ipa") for p in entry.get("pronunciations") or []):
            fields["ipa"] += 1
        for key in ("examples", "translations", "synonyms", "collocations"):
            if any(s.get(key) for s in senses):
                fields[key] += 1

    terms = dict(conn.execute("SELECT term_type, count(*) FROM search_terms GROUP BY 1"))
    langs = [r[0] for r in conn.execute("SELECT DISTINCT language FROM entries")]

    print(f"\n[coverage] {db_path} — {total:,} entries, languages {langs}")
    for key, hits in fields.items():
        print(f"    {key:13s} {hits:8,}  {100.0*hits/total:5.1f}%")
    print(f"    search terms  {terms}")

    warnings = 0
    if target in EXPECT_TRANSLATIONS and fields["translations"] == 0:
        print("[warn] 0% translations — this edition should carry them; wrong dump?")
        warnings += 1
    if target == "zh":
        cjk = sum(
            1
            for (lemma,) in conn.execute("SELECT lemma FROM entries")
            if any("一" <= ch <= "鿿" for ch in lemma)
        )
        share = 100.0 * cjk / total
        print(f"    CJK lemmas    {cjk:8,}  {share:5.1f}%")
        if share < 50:
            print("[warn] a Chinese pack with <50% Han lemmas is the romanization index")
            warnings += 1
        if not terms.get("transcription"):
            print("[warn] zero transcription rows — pinyin lookup will not work")
            warnings += 1
    conn.close()
    return 0


def main() -> None:
    target = sys.argv[1] if len(sys.argv) > 1 else "en"
    max_entries = sys.argv[2] if len(sys.argv) > 2 else None

    if target not in SOURCES:
        print(f"Unknown target '{target}'. Available: {', '.join(SOURCES)}")
        sys.exit(1)

    cfg = SOURCES[target]
    os.makedirs("fixtures", exist_ok=True)
    os.makedirs("packs", exist_ok=True)

    pack_tool = build_cli()
    raw = raw_path(target)
    download_file(cfg["url"], raw)
    manifest = write_manifest(target, cfg, raw)

    print(f"[build] {raw} -> {cfg['output']}")
    cmd = [pack_tool, "kaikki", manifest, raw, cfg["output"], "kaikki-wiktionary"]
    if max_entries:
        cmd.append(max_entries)
    res = subprocess.run(cmd)
    if res.returncode != 0:
        print(f"[error] pack build failed ({res.returncode})", file=sys.stderr)
        sys.exit(res.returncode)

    sys.exit(report_coverage(cfg["output"], target))


if __name__ == "__main__":
    main()
