import os
import sys
import time
import urllib.request
import subprocess

SOURCES = {
    "en": {
        "name": "English (Core)",
        "url": "https://kaikki.org/simplewiktionary/English/kaikki.org-dictionary-English.jsonl",
        "manifest": "fixtures/kaikki-en.manifest.json",
        "raw": "fixtures/kaikki-simple-en.jsonl",
        "output": "packs/kaikki-english-core.sqlite",
    },
    "ary": {
        "name": "Moroccan Arabic (Darija)",
        "url": "https://kaikki.org/dictionary/Moroccan%20Arabic/kaikki.org-dictionary-MoroccanArabic.jsonl",
        "manifest": "fixtures/kaikki-ary.manifest.json",
        "raw": "fixtures/kaikki-ary.jsonl",
        "output": "packs/kaikki-darija.sqlite",
    },
    "zh": {
        "name": "Chinese (Mandarin)",
        "url": "https://kaikki.org/dictionary/Mandarin/kaikki.org-dictionary-Mandarin.jsonl",
        "manifest": "fixtures/kaikki-zh.manifest.json",
        "raw": "fixtures/kaikki-mandarin.jsonl",
        "output": "packs/kaikki-mandarin.sqlite",
    },
    "ca": {
        "name": "Catalan",
        "url": "https://kaikki.org/dictionary/Catalan/kaikki.org-dictionary-Catalan.jsonl",
        "manifest": "fixtures/kaikki-ca.manifest.json",
        "raw": "fixtures/kaikki-ca.jsonl",
        "output": "packs/kaikki-catalan.sqlite",
    },
    "es": {
        "name": "Spanish",
        "url": "https://kaikki.org/dictionary/Spanish/kaikki.org-dictionary-Spanish.jsonl",
        "manifest": "fixtures/kaikki-es.manifest.json",
        "raw": "fixtures/kaikki-es.jsonl",
        "output": "packs/kaikki-spanish.sqlite",
    },
    "fr": {
        "name": "French",
        "url": "https://kaikki.org/dictionary/French/kaikki.org-dictionary-French.jsonl",
        "manifest": "fixtures/kaikki-fr.manifest.json",
        "raw": "fixtures/kaikki-fr.jsonl",
        "output": "packs/kaikki-french.sqlite",
    },
    "de": {
        "name": "German",
        "url": "https://kaikki.org/dictionary/German/kaikki.org-dictionary-German.jsonl",
        "manifest": "fixtures/kaikki-de.manifest.json",
        "raw": "fixtures/kaikki-de.jsonl",
        "output": "packs/kaikki-german.sqlite",
    },
}

def download_file(url: str, dest: str):
    if os.path.exists(dest) and os.path.getsize(dest) > 1024:
        print(f"[CACHE] {dest} already exists ({os.path.getsize(dest)/(1024*1024):.1f} MB), skipping download.")
        return
    print(f"[DOWNLOAD] Fetching {url} -> {dest}...")
    t0 = time.time()
    urllib.request.urlretrieve(url, dest)
    dt = time.time() - t0
    print(f"[DOWNLOAD] Finished in {dt:.1f}s ({os.path.getsize(dest)/(1024*1024):.1f} MB).")

def main():
    target = sys.argv[1] if len(sys.argv) > 1 else "en"
    if target not in SOURCES:
        print(f"Unknown target: {target}. Available targets: {list(SOURCES.keys())}")
        sys.exit(1)

    cfg = SOURCES[target]
    os.makedirs("fixtures", exist_ok=True)
    os.makedirs("packs", exist_ok=True)

    download_file(cfg["url"], cfg["raw"])

    pack_tool = os.path.join("src-tauri", "target", "release", "golddig-pack.exe")
    if not os.path.exists(pack_tool):
        pack_tool = os.path.join("src-tauri", "target", "debug", "golddig-pack.exe")

    print(f"[BUILD] Compiling pack {cfg['output']} from {cfg['raw']}...")
    cmd = [pack_tool, "kaikki", cfg["manifest"], cfg["raw"], cfg["output"]]
    res = subprocess.run(cmd)
    if res.returncode == 0:
        print(f"[SUCCESS] Pack generated successfully at {cfg['output']}!")
    else:
        print(f"[ERROR] Pack compilation failed with code {res.returncode}")
        sys.exit(res.returncode)

if __name__ == "__main__":
    main()
